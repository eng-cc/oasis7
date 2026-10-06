#!/usr/bin/env python3
"""Focused fake-adapter coverage for C1 ordering, recovery, and resolution."""
from __future__ import annotations

import copy
import hashlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch
from urllib.parse import urlencode

ROOT = Path(__file__).parent
sys.path.insert(0, str(ROOT))

import pr_projection_journal as journal_module
import pr_projection_publication as publication_module
import pr_projection_publish as publish_module
import pr_projection_resolver as resolver_module
from projection_publication_contract import digest, encode_marker

UID = "task_" + "a" * 32
CONFIG = "sha256:" + "c" * 64
SCOPE = "b" * 40
AUTHORITY = "d" * 40


def make_publication(index: int, *, head: str | None = None, branch: str | None = None,
                     repository: str = "eng-cc/oasis7", task_uid: str = UID):
    head = head or f"{index + 1:040x}"
    branch = branch or f"feature/c1-{index}"
    projection_digest = digest({"target": index})
    publication = publication_module.build_task_publication(
        repository=repository, repository_id=7, task_uid=task_uid,
        bootstrap_epoch=1, source_repository_id=7, source_ref=branch,
        target_ref="main", source_head_oid=head, source_scope_oid=SCOPE,
        planner_authority_oid=AUTHORITY, planner_config_sha256=CONFIG,
        policy_digest=digest({"policy": "test"}), projection_digest=projection_digest,
    )
    projection = {
        "task_uid": task_uid, "source_head_oid": head, "scope_base_oid": SCOPE,
        "planner_config_sha256": CONFIG, "projection_digest": projection_digest,
        "consumed_contracts": [],
    }
    return publication, projection


class FakeAdapter:
    def __init__(self, publication, projection, *, initial_head=None, initial_pr=None):
        self.publications = []
        self.publication_author_logins = {}
        self.publisher_login = "oasis7-test-publisher"
        self.issue_number = 1
        self.bindings = []
        self.pr_binding = None
        self.events = []
        self.source_ref = initial_head
        self.ready_update_admitted = False
        self.recovery_admission_available = True
        self.recovery_surface_state = "all_pre"
        self.metadata_writes = []
        self.projection = projection
        self.prs = []
        if initial_pr:
            self.prs.append(copy.deepcopy(initial_pr))
            self.pr_binding = {"task_uid": UID, "pr_number": initial_pr["number"]}

    def find_task_publications(self, pub_id):
        publications = [p for p in self.publications if p["publication_id"] == pub_id]
        return {
            "complete": True,
            "publications": publications,
            "publication_authors": [
                {"publication_id": item["publication_id"],
                 "author_login": self.publication_author_logins.get(
                     item["publication_id"], self.publisher_login,
                 )}
                for item in publications
            ],
            "publication_bodies": [
                {"publication_id": item["publication_id"],
                 "body": publication_module.publication_comment(item)}
                for item in publications
            ],
        }

    def resolve_publisher_login(self):
        self.events.append("resolve-publisher")
        return self.publisher_login

    def publish_task_intent(self, value):
        self.events.append("task-intent")
        self.publications.append(copy.deepcopy(value))
        self.publication_author_logins[value["publication_id"]] = self.publisher_login

    def read_source_ref(self, ref):
        self.events.append("read-source")
        return self.source_ref

    def push_source_ref(self, ref, new_oid, lease_oid):
        self.events.append("push")
        if self.source_ref != lease_oid:
            raise RuntimeError("lease changed")
        self.source_ref = new_oid
        for pr in self.prs:
            pr["head_oid"] = new_oid

    def find_task_prs(self, task_uid, source_ref, target_ref):
        self.events.append("discover-pr")
        return {"complete": True, "pull_requests": [
            pr for pr in self.prs
            if pr["source_ref"] == source_ref and pr["target_ref"] == target_ref
        ]}

    def create_draft_pr(self, repository, source_ref, target_ref, body):
        self.events.append("create-pr")
        pr = {"repository": repository, "source_ref": source_ref, "target_ref": target_ref,
              "head_oid": self.source_ref, "body": body, "state": "open", "merged": False,
              "draft": True, "number": len(self.prs) + 1}
        self.prs.append(pr)

    def read_task_pr_binding(self, task_uid):
        self.events.append("read-task-pr-binding")
        binding = copy.deepcopy(self.pr_binding or {"task_uid": task_uid, "pr_number": None})
        if self.ready_update_admitted:
            binding["existing_ready_update"] = True
        return binding

    def record_pr(self, task_uid, number, publication_id):
        self.events.append("record-pr")
        self.metadata_writes.extend(("project", "issue", "mapping"))
        self.pr_binding = {"task_uid": task_uid, "pr_number": number}

    def require_record_pr_recovery_admission(self):
        self.events.append("check-record-pr-recovery-admission")
        if not self.recovery_admission_available:
            raise publication_module.PublicationError(
                "NETWORK_UNCERTAIN",
                f"recovery admission required for {self.recovery_surface_state}",
            )

    def find_task_publication_bindings(self, pub_id):
        self.events.append("read-reciprocal")
        return {"complete": True, "bindings": [
            item for item in self.bindings if item["publication_id"] == pub_id
        ]}

    def publish_reciprocal_binding(self, value):
        self.events.append("publish-reciprocal")
        self.bindings.append(copy.deepcopy(value))

    def read_pr(self, repository, number):
        self.events.append("read-pr")
        return copy.deepcopy(next(pr for pr in self.prs if pr["number"] == number))

    def patch_pr_body(self, repository, number, body):
        self.events.append("patch-pr")
        next(pr for pr in self.prs if pr["number"] == number)["body"] = body


class FirstPublicationReadTimeoutAdapter(FakeAdapter):
    def __init__(self, publication, projection, *, initial_head, initial_pr):
        super().__init__(publication, projection, initial_head=initial_head, initial_pr=initial_pr)
        self.pr_binding = None
        self.publication_reads = 0

    def find_task_publications(self, pub_id):
        self.publication_reads += 1
        self.events.append("read-task-publications")
        if self.publication_reads == 1:
            raise TimeoutError("simulated prepublication read timeout")
        return super().find_task_publications(pub_id)


class FakeClock:
    def __init__(self):
        self.now = 0.0
        self.sleeps = []

    def clock(self):
        return self.now

    def sleep(self, duration):
        self.sleeps.append(duration)
        self.now += duration


class BoundedRecoveryCLITests(unittest.TestCase):
    """Use the real record-pr CLI; only the shell fixture replaces GitHub IO.

    Journals, ancestor/current Git commits and publication comments are built
    by production code in each disposable fixture. No FakeAdapter.record_pr
    implementation stands in for the recovery boundary.
    """

    def run_case(self, case, *, timeout=30):
        environment = dict(os.environ, OASIS7_REC_RED_ONLY="1", OASIS7_REC_CASE=case)
        result = subprocess.run(
            ["bash", str(ROOT / "github-project-task.test.sh")],
            cwd=ROOT.parents[1], env=environment, capture_output=True,
            text=True, timeout=timeout,
        )
        self.assertEqual(0, result.returncode,
                         f"REC actual CLI case={case} exit={result.returncode}\n"
                         + result.stdout + result.stderr)

    def fixture_journal(self, common_dir, publication):
        return journal_module.open_journal(
            common_dir, publication["repository"], publication["source_ref"],
            publication["publication_id"], task_uid=publication["task_uid"],
            source_head_oid=publication["source_head_oid"],
            scope_base_oid=publication["source_scope_oid"],
            projection_digest=publication["projection_digest"],
        )

    def existing_uncertain_record_pr_case(self, temp, index, *, surface_state,
                                          uncertain_record_pr=True):
        publication, projection = make_publication(index)
        _, marker = publication_module.prepare(
            task_uid=UID, source_head_oid=publication["source_head_oid"],
            scope_base_oid=publication["source_scope_oid"],
            projection_digest=publication["projection_digest"],
        )
        body = publication_module.replace_projection_marker(
            f"Task: {UID}\nRefs #1", marker,
        )
        pr = {
            "repository": publication["repository"], "source_ref": publication["source_ref"],
            "target_ref": publication["target_ref"], "head_oid": publication["source_head_oid"],
            "body": body, "state": "open", "merged": False,
            "draft": True, "number": index,
        }
        adapter = FakeAdapter(
            publication, projection,
            initial_head=publication["source_head_oid"], initial_pr=pr,
        )
        adapter.recovery_admission_available = False
        adapter.recovery_surface_state = surface_state
        adapter.publications.append(copy.deepcopy(publication))
        adapter.publication_author_logins[publication["publication_id"]] = adapter.publisher_login
        journal = self.fixture_journal(temp, publication)
        if uncertain_record_pr:
            action = "record-pr:" + publication["publication_id"]
            with journal.locked():
                journal.intent(action, "record_pr", {
                    "publication_id": publication["publication_id"],
                    "task_uid": UID, "pr_number": index,
                })
                journal.uncertain(action, "NETWORK_UNCERTAIN")
        return publication, projection, adapter, journal, f"Task: {UID}\nRefs #1\n"

    def test_pending_record_pr_without_admission_blocks_project_post_issue_pre(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal, body = self.existing_uncertain_record_pr_case(
                temp, 7240, surface_state="Project-post/Issue-pre",
            )
            error = None
            try:
                publication_module.publish_create(
                    adapter, journal, publication=publication,
                    projection=projection, body=body,
                    expected_remote_oid=publication["source_head_oid"],
                )
            except publication_module.PublicationError as exc:
                error = exc

            self.assertIsNotNone(
                error,
                "uncertain Project-post/Issue-pre retry entered record-pr without admission; "
                f"events={adapter.events!r}; metadata_writes={adapter.metadata_writes!r}",
            )
            self.assertIn("recovery admission", str(error))
            self.assertEqual([], adapter.metadata_writes)
            self.assertNotIn("task-intent", adapter.events,
                             "exact readback must not turn recovery into another Task POST")
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_pending_record_pr_without_admission_blocks_exact_poststate(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal, body = self.existing_uncertain_record_pr_case(
                temp, 7241, surface_state="Issue-and-Project-exact-poststate",
            )
            error = None
            try:
                publication_module.publish_create(
                    adapter, journal, publication=publication,
                    projection=projection, body=body,
                    expected_remote_oid=publication["source_head_oid"],
                )
            except publication_module.PublicationError as exc:
                error = exc

            self.assertIsNotNone(
                error,
                "uncertain exact-poststate retry entered record-pr without admission; "
                f"events={adapter.events!r}; metadata_writes={adapter.metadata_writes!r}",
            )
            self.assertIn("recovery admission", str(error))
            self.assertEqual([], adapter.metadata_writes)
            self.assertNotIn("task-intent", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_first_record_pr_without_admission_keeps_all_pre_path(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal, body = self.existing_uncertain_record_pr_case(
                temp, 7242, surface_state="Issue-and-Project-all-pre",
                uncertain_record_pr=False,
            )
            result = publication_module.publish_create(
                adapter, journal, publication=publication,
                projection=projection, body=body,
                expected_remote_oid=publication["source_head_oid"],
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(1, adapter.events.count("record-pr"))
            self.assertEqual(["project", "issue", "mapping"], adapter.metadata_writes)
            self.assertNotIn("check-record-pr-recovery-admission", adapter.events)

    def legacy_anchor_lineage_case(self, temp):
        root = Path(temp).resolve()
        root.mkdir(parents=True, exist_ok=True)

        def git(*args):
            return subprocess.check_output(
                ["git", "-C", str(root), *args], text=True,
            ).strip()

        subprocess.run(["git", "-C", str(root), "init", "-q"], check=True)
        git("config", "user.name", "Publication Recovery Fixture")
        git("config", "user.email", "publication-recovery-fixture@example.test")
        (root / "lineage.txt").write_text("base\n", encoding="utf-8")
        git("add", "lineage.txt")
        git("commit", "-q", "-m", "base")
        base_oid = git("rev-parse", "HEAD")
        (root / "lineage.txt").write_text("current\n", encoding="utf-8")
        git("add", "lineage.txt")
        git("commit", "-q", "-m", "current")
        current_oid = git("rev-parse", "HEAD")

        def build(head, label):
            return publication_module.build_task_publication(
                repository="eng-cc/oasis7", repository_id=7, task_uid=UID,
                bootstrap_epoch=1, source_repository_id=7,
                source_ref="feature/publication-recovery", target_ref="main",
                source_head_oid=head, source_scope_oid=base_oid,
                planner_authority_oid=base_oid, planner_config_sha256=CONFIG,
                policy_digest=digest({"policy": "legacy-anchor-fixture"}),
                projection_digest=digest({"publication": label}),
            )

        previous = build(base_oid, "previous")
        current = build(current_oid, "current")
        old_journal = self.fixture_journal(root / ".git", previous)
        current_journal = self.fixture_journal(root / ".git", current)
        for journal, item in ((old_journal, previous), (current_journal, current)):
            action = "record-pr:" + item["publication_id"]
            with journal.locked():
                journal.intent(action, "record_pr", {
                    "publication_id": item["publication_id"],
                    "task_uid": UID, "pr_number": 2001,
                })
                journal.uncertain(action, "NETWORK_UNCERTAIN")
                if item is current:
                    journal.intent(
                        "task-intent:" + item["publication_id"],
                        "publish_task_intent",
                        {"publication_id": item["publication_id"], "task_uid": UID},
                    )
                    state = journal.read_action_state()
                    state.pop("task_post_tail", None)
                    journal_module._atomic_json(journal.path, state)

        preanchor_raw = current_journal.path.read_bytes()
        adapter = FakeAdapter(current, {"consumed_contracts": []}, initial_head=current_oid)
        adapter.root = root
        adapter.publications.append(copy.deepcopy(current))
        adapter.publication_author_logins[current["publication_id"]] = adapter.publisher_login
        binding = publication_module.build_publication_binding(
            current, 2001, "https://github.com/eng-cc/oasis7/pull/2001",
        )

        def issue_comment(comment_id, item):
            body = publication_module.publication_comment(item)
            return {
                "id": comment_id, "body": body,
                "user": {"login": adapter.publisher_login},
                "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/1",
                "html_url": f"https://github.com/eng-cc/oasis7/issues/1#issuecomment-{comment_id}",
            }

        comments = [issue_comment(1001, previous), issue_comment(1002, current)]

        def lineage_evidence(item, comment_id, journal, raw):
            body = comments[comment_id - 1001]["body"]
            return {
                "publication_id": item["publication_id"],
                "action_id": "record-pr:" + item["publication_id"],
                "journal_sha256": hashlib.sha256(raw).hexdigest(),
                "H": item["source_head_oid"], "B": item["planner_authority_oid"],
                "S": item["source_scope_oid"], "D": item["projection_digest"],
                "intent_comment_id": comment_id,
                "intent_body_sha256": hashlib.sha256(body.encode("utf-8")).hexdigest(),
            }

        old_raw = old_journal.path.read_bytes()
        module_path = ROOT / "github-project-task.py"
        spec = importlib.util.spec_from_file_location("publication_recovery_task_helper", module_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        authority = object.__new__(helper.PublicationRecoveryAuthority)
        authority.root = root
        authority.args = type("Args", (), {
            "root": root, "repo": "eng-cc/oasis7", "task_uid": UID,
        })()
        authority.record = {
            "issue_number": 1, "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
        }
        authority.intent = current
        authority.binding = binding
        authority.module = publication_module
        authority.comment = {"user": {"login": adapter.publisher_login}}
        # Production recovery instances initialize this scratch anchor in
        # __init__; this isolated lineage test intentionally uses __new__ to
        # exercise the exact historical journal state without the live API
        # admission setup.
        authority._current_journal_expected_raw = None
        authority.issue_baseline = None
        authority.project_baseline = None
        authority._record_pr_start_vectors = None
        authority._record_pr_latest_vectors = None
        authority._admitted_issue_body = None
        authority._current_issue_expected_body = None
        authority._expected_default_merge_hold = None
        authority.envelope = {
            "current_action": lineage_evidence(current, 1002, current_journal, preanchor_raw),
            "predecessor": lineage_evidence(previous, 1001, old_journal, old_raw),
        }
        return {
            "adapter": adapter, "authority": authority, "comments": comments,
            "current": current, "current_journal": current_journal,
            "preanchor_raw": preanchor_raw,
        }

    def resolve_legacy_task_post_without_post(self, case, *, pr_binding=None):
        current = case["current"]
        action = "task-intent:" + current["publication_id"]
        with case["current_journal"].locked():
            publication_module._intent(
                case["adapter"], case["current_journal"], current,
                pr_binding=(pr_binding if pr_binding is not None else
                            {"task_uid": UID, "pr_number": 2001}),
                resume_action_id=action,
            )
            events = case["current_journal"].read_task_events(action)
        return events

    def test_exact_legacy_task_readback_anchor_migration_preserves_admitted_lineage(self):
        with tempfile.TemporaryDirectory() as temp:
            case = self.legacy_anchor_lineage_case(temp)
            events = self.resolve_legacy_task_post_without_post(
                case, pr_binding={"state": "unbound"},
            )
            self.assertEqual(["READ_MATCH", "RESOLVED"], [item["event"] for item in events])
            self.assertEqual(0, case["adapter"].events.count("task-intent"))
            self.assertNotEqual(case["preanchor_raw"], case["current_journal"].path.read_bytes())

            try:
                case["authority"]._lineage(case["comments"])
            except ValueError as exc:
                self.fail(
                    "an admitted exact Task readback may migrate only its verified legacy root tail; "
                    f"lineage was rejected: {exc}"
                )

    def test_exact_legacy_task_readback_bound_producer_shape_preserves_lineage(self):
        with tempfile.TemporaryDirectory() as temp:
            case = self.legacy_anchor_lineage_case(temp)
            events = self.resolve_legacy_task_post_without_post(
                case, pr_binding={"state": "bound", "pr_number": 2001},
            )
            self.assertEqual(["READ_MATCH", "RESOLVED"], [item["event"] for item in events])
            try:
                case["authority"]._lineage(case["comments"])
            except ValueError as exc:
                self.fail(f"exact producer-bound Task readback should preserve lineage: {exc}")

    def test_exact_legacy_task_readback_malformed_pr_binding_stays_rejected(self):
        malformed_bindings = (
            {"state": "unbound", "unexpected": True},
            {"state": "bound", "pr_number": 2002},
        )
        for binding in malformed_bindings:
            with self.subTest(binding=binding), tempfile.TemporaryDirectory() as temp:
                case = self.legacy_anchor_lineage_case(temp)
                events = self.resolve_legacy_task_post_without_post(case, pr_binding=binding)
                self.assertEqual(["READ_MATCH", "RESOLVED"], [item["event"] for item in events])
                with self.assertRaisesRegex(ValueError, "raw publication journal hash mismatch"):
                    case["authority"]._lineage(case["comments"])

    def test_exact_legacy_task_readback_malformed_tail_stays_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            case = self.legacy_anchor_lineage_case(temp)
            events = self.resolve_legacy_task_post_without_post(case)
            self.assertEqual(["READ_MATCH", "RESOLVED"], [item["event"] for item in events])
            with case["current_journal"].locked():
                state = case["current_journal"].read_action_state()
                state["task_post_tail"]["digest"] = "sha256:" + "f" * 64
                journal_module._atomic_json(case["current_journal"].path, state)
                with self.assertRaisesRegex(journal_module.JournalError, "anchor"):
                    case["current_journal"].read_task_events()

    def test_exact_legacy_task_readback_sidecar_sequence_drift_stays_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            case = self.legacy_anchor_lineage_case(temp)
            events = self.resolve_legacy_task_post_without_post(case)
            self.assertEqual(["READ_MATCH", "RESOLVED"], [item["event"] for item in events])
            with case["current_journal"].locked():
                lines = case["current_journal"].task_events_path.read_text().splitlines()
                first = json.loads(lines[0])
                first["sequence"] = 99
                lines[0] = json.dumps(first, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
                case["current_journal"].task_events_path.write_text("\n".join(lines) + "\n")
                with self.assertRaisesRegex(journal_module.JournalError, "sequence"):
                    case["current_journal"].read_task_events()

    def test_unrelated_current_journal_drift_does_not_gain_f1_lineage(self):
        with tempfile.TemporaryDirectory() as temp:
            case = self.legacy_anchor_lineage_case(temp)
            events = self.resolve_legacy_task_post_without_post(case)
            self.assertEqual(["READ_MATCH", "RESOLVED"], [item["event"] for item in events])
            with case["current_journal"].locked():
                state = case["current_journal"].read_action_state()
                state["unrelated_metadata"] = "tampered"
                journal_module._atomic_json(case["current_journal"].path, state)
            with self.assertRaisesRegex(ValueError, "raw publication journal hash mismatch"):
                case["authority"]._lineage(case["comments"])

    def test_recovery_record_pr_budget_covers_required_fresh_checks(self):
        # Reuse the genuine immutable closure/three-role authority fixture and
        # real publisher/record-pr. Scale only a subprocess deadline probe;
        # the actual CLI still runs with its production aggregate budget.
        fixture = (ROOT / "github-project-task.test.sh").read_text()
        anchor = "import pr_projection_publish as publisher\nroot, uid, case = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]"
        self.assertEqual(1, fixture.count(anchor))
        probe = """import subprocess
original_command_output = publisher.command_output
def deadline_probe(command, *, timeout=publisher.LOCAL_COMMAND_TIMEOUT_SECONDS,
                   reservation_fd=None):
    if "record-pr" in command:
        # 0.8s represents an 80s barrier workload: too long for legacy60,
        # within recovery180. A real child process enforces the deadline.
        subprocess.run([sys.executable, "-c", "import time; time.sleep(0.8)"],
            check=True, timeout=timeout / 100)
    return original_command_output(command, timeout=timeout, reservation_fd=reservation_fd)
publisher.command_output = deadline_probe
"""
        with tempfile.TemporaryDirectory() as temp:
            isolated = Path(temp) / "budget-fixture.sh"
            isolated.write_text(fixture.replace(anchor, "import pr_projection_publish as publisher\n" + probe + anchor.split("\n", 1)[1]))
            environment = dict(os.environ, PM_ROOT_DIR=str(ROOT.parents[1]),
                OASIS7_REC_RED_ONLY="1", OASIS7_REC_CASE="idempotent_repeat")
            result = subprocess.run(["bash", str(isolated)], cwd=ROOT.parents[1],
                env=environment, capture_output=True, text=True, timeout=60)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("PASS test_rec_idempotent_repeat", result.stdout)

    def test_c1_writer_rechecks_scope_after_recovery_admission(self):
        self.run_case("guard_c1_writer_scope_drift")

    def test_c1_writer_rejects_unknown_journal_postimage(self):
        self.run_case("guard_c1_writer_unknown_journal_drift")

    def test_c1_writer_rejects_reordered_journal_actions(self):
        self.run_case("guard_c1_writer_reordered_journal_drift")

    def test_c1_writer_rejects_step_without_vector(self):
        self.run_case("guard_c1_writer_step_without_vector")

    def test_c1_writer_rejects_unreachable_step_suffix(self):
        self.run_case("guard_c1_writer_unreachable_step_suffix")

    def test_c1_writer_rejects_binding_comment_without_predecessors(self):
        self.run_case("guard_c1_writer_binding_comment_without_predecessors")

    def test_c1_writer_rejects_uncertain_step_with_observation(self):
        self.run_case("guard_c1_writer_uncertain_step_with_observation")

    def test_c1_writer_rejects_observed_target_vector_when_live_is_before(self):
        self.run_case("guard_c1_writer_observed_vector_live_before")

    def test_actual_interrupted_retries_keep_multiple_pending_steps_reachable(self):
        self.run_case("interrupted_retry_accumulates_pending_steps", timeout=90)

    def test_issue_step_binds_full_body_before_effect_and_rejects_later_body_drift(self):
        self.run_case("guard_issue_body_proof_after_write", timeout=90)

    def test_child_cannot_fall_back_after_recovery_admission_is_revoked(self):
        # Model the narrow launch race in an isolated copy of the shell fixture:
        # parent preflight and helper selection see the marker, then the fake
        # GitHub endpoint removes it on the real child CLI's first fresh read.
        # Make Issue, Project, and mapping agree on the ordinary all-pre state
        # so fallback would otherwise be eligible to write.
        fixture = (ROOT / "github-project-task.test.sh").read_text()
        comment_anchor = 'case = sys.argv[2]\nimport os\nscope_drift_armed ='
        self.assertEqual(1, fixture.count(comment_anchor))
        comment_hook = '''case = sys.argv[2]
if case == "revoke_admission_after_child_launch" and __import__("os").environ.get("GH_REC_CHILD_LAUNCHED_FILE"):
    launched = pathlib.Path(__import__("os").environ["GH_REC_CHILD_LAUNCHED_FILE"])
    if launched.is_file():
        counter = directory.parent / "gh-comment-read-count.txt"
        reads = int(counter.read_text()) if counter.exists() else 0
        reads += 1
        counter.write_text(str(reads))
        if reads == 1:
            admission = directory / "9001"
            assert admission.is_file(), "recovery admission was already absent before child read"
            admission.unlink()
            (directory.parent / "admission-revoked-during-child-read.txt").write_text("1\\n")
import os
scope_drift_armed ='''
        fixture = fixture.replace(comment_anchor, comment_hook)

        recovery_case_anchor = 'if [[ "$REC_CASE" == "pending_final_readback" || "$REC_CASE" == "pending_project_content_drift" || "$REC_CASE" == "idempotent_repeat" ]]; then'
        self.assertEqual(1, fixture.count(recovery_case_anchor))
        fixture = fixture.replace(
            recovery_case_anchor,
            recovery_case_anchor.replace(
                '"idempotent_repeat"',
                '"idempotent_repeat" || "$REC_CASE" == "revoke_admission_after_child_launch"',
            ),
        )

        all_pre_anchor = '    pr = adapter.read_pr(value["repository"], 2001)\n'
        self.assertEqual(1, fixture.count(all_pre_anchor))
        all_pre_setup = '''    pr = adapter.read_pr(value["repository"], 2001)
    if case == "revoke_admission_after_child_launch":
        import os
        mapping_path = root / ".pm/github-project-sync/tasks.json"
        mapping = json.loads(mapping_path.read_text())
        mapping["tasks"][uid]["workflow_phase"] = "execution"
        mapping_path.write_text(json.dumps(mapping))
        (root / "project-live-phase").write_text("execution\\n")
        (root / "project-pr.md").write_text("")
        (root / "revocation-mapping-before.json").write_bytes(mapping_path.read_bytes())
        (root / "revocation-issue-before.txt").write_bytes((root / "issue-live-body.md").read_bytes())
        child_launched = root / "record-pr-child-launched.txt"
        os.environ["GH_REC_CHILD_LAUNCHED_FILE"] = str(child_launched)
        record_pr_globals = adapter.record_pr.__func__.__globals__
        original_command_output = record_pr_globals["command_output"]
        def mark_record_pr_child_launch(command, *, timeout=publisher.LOCAL_COMMAND_TIMEOUT_SECONDS,
                                        reservation_fd=None):
            child_launched.write_text("launched\\n")
            return original_command_output(command, timeout=timeout, reservation_fd=reservation_fd)
        record_pr_globals["command_output"] = mark_record_pr_child_launch
'''
        fixture = fixture.replace(all_pre_anchor, all_pre_setup)

        revocation_branch_anchor = '    else:\n        publication._record_and_bind(adapter, local, value, pr)\n'
        self.assertEqual(1, fixture.count(revocation_branch_anchor))
        revocation_branch = '''    elif case == "revoke_admission_after_child_launch":
        calls_path = root / "gh-calls.log"
        calls_before = len(calls_path.read_text().splitlines())
        issue_before = (root / "issue-live-body.md").read_bytes()
        phase_before = (root / "project-live-phase").read_bytes()
        project_pr_before = (root / "project-pr.md").read_bytes()
        error = None
        try:
            publication._record_and_bind(adapter, local, value, pr)
        except publication.PublicationError as exc:
            error = exc
        assert error is not None and error.code == "NETWORK_UNCERTAIN", error
        revoked = root / "admission-revoked-during-child-read.txt"
        assert revoked.is_file(), (
            "child fresh read did not revoke admission; "
            f"reads={(root / 'gh-comment-read-count.txt').read_text() if (root / 'gh-comment-read-count.txt').exists() else 'missing'}; "
            f"launched={(root / 'record-pr-child-launched.txt').exists()}; "
            f"calls={calls_path.read_text()}"
        )
        assert revoked.read_text() == "1\\n"
        assert (root / "record-pr-child-launched.txt").is_file(), "real record-pr child was not launched"
        calls = [line.replace("\\\\ ", " ") for line in calls_path.read_text().splitlines()[calls_before:]]
        writes = [line for line in calls if line.startswith(("issue edit ", "issue comment ", "project item-edit "))]
        assert not writes, "revoked recovery fell back to ordinary metadata writes: " + repr(writes)
        assert (root / ".pm/github-project-sync/tasks.json").read_bytes() == (root / "revocation-mapping-before.json").read_bytes(), "revoked recovery changed mapping"
        assert (root / "issue-live-body.md").read_bytes() == issue_before, "revoked recovery changed Issue"
        assert (root / "project-live-phase").read_bytes() == phase_before, "revoked recovery changed Project phase"
        assert (root / "project-pr.md").read_bytes() == project_pr_before, "revoked recovery changed Project PR"
        assert not (root / "gh-comments/9001").exists(), "fixture failed to remove the admission marker"
        print("PASS test_rec_revoke_admission_after_child_launch")
    else:
        publication._record_and_bind(adapter, local, value, pr)
'''
        fixture = fixture.replace(revocation_branch_anchor, revocation_branch)

        with tempfile.TemporaryDirectory() as temp:
            isolated = Path(temp) / "revocation-fixture.sh"
            isolated.write_text(fixture)
            environment = dict(os.environ, PM_ROOT_DIR=str(ROOT.parents[1]),
                OASIS7_REC_RED_ONLY="1", OASIS7_REC_CASE="revoke_admission_after_child_launch")
            result = subprocess.run(["bash", str(isolated)], cwd=ROOT.parents[1],
                env=environment, capture_output=True, text=True, timeout=60)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("PASS test_rec_revoke_admission_after_child_launch", result.stdout)

    def test_project_post_issue_pre_cache_post_reconciles(self):
        self.run_case("project_post_issue_pre")

    def test_project_pre_issue_post_reconciles(self):
        self.run_case("project_pre_issue_post")

    def test_all_pre_reconciles_with_final_readback(self):
        self.run_case("all_pre")

    def test_all_post_reconciles_with_final_readback(self):
        self.run_case("all_post")

    def test_mixed_issue_lifecycle_vector_is_rejected_without_writes(self):
        self.run_case("guard_fieldwise_issue_mixed")

    def test_pre_cache_cannot_replace_final_authoritative_readback(self):
        self.run_case("cache_pre_project_post")

    def test_missing_old_journal_rejected_without_writes(self):
        self.run_case("guard_missing_old_journal")

    def test_old_observed_action_is_not_uncertain_lineage(self):
        self.run_case("guard_old_not_uncertain")

    def test_duplicate_current_intent_rejected_without_writes(self):
        self.run_case("guard_duplicate_current_intent")

    def test_missing_current_intent_rejected_without_writes(self):
        self.run_case("guard_missing_current_intent")

    def test_unrelated_issue_drift_rejected_without_writes(self):
        self.run_case("guard_unrelated_issue_drift")

    def test_repository_field_identity_drift_rejected_without_writes(self):
        self.run_case("guard_repository_identity_drift")

    def test_repository_field_malformed_identity_rejected_without_writes(self):
        self.run_case("guard_repository_identity_malformed")

    def test_project_item_content_must_be_exact_task_issue_before_writes(self):
        for case in (
            "guard_project_item_content_wrong",
            "guard_project_item_content_missing",
            "guard_project_item_content_nonissue",
            "guard_project_item_content_cross_repository",
        ):
            with self.subTest(case=case):
                self.run_case(case)

    def test_project_item_content_is_rechecked_after_each_project_write(self):
        self.run_case("guard_project_item_content_late_drift")

    def test_unrelated_project_drift_rejected_without_writes(self):
        self.run_case("guard_unrelated_project_drift")

    def test_live_pr_head_drift_rejected_without_writes(self):
        self.run_case("guard_pr_head_drift")

    def test_live_pr_task_refs_drift_rejected_without_writes(self):
        self.run_case("guard_pr_task_refs_drift")

    def test_final_issue_read_failure_retains_current_pending_action(self):
        self.run_case("pending_final_readback")

    def test_repeated_actual_publisher_converges_idempotently(self):
        self.run_case("idempotent_repeat")

    def test_final_project_content_drift_retains_current_pending_action(self):
        self.run_case("pending_project_content_drift")

    def test_old_action_pr_tuple_mismatch_rejected_without_writes(self):
        self.run_case("guard_old_action_tuple")

    def test_current_journal_identity_digest_mismatch_rejected_without_writes(self):
        self.run_case("guard_current_journal_identity")

    def test_authentic_old_publication_nonancestor_rejected_without_writes(self):
        self.run_case("guard_old_nonancestor")

    def test_current_issue_write_permission_required_before_writes(self):
        self.run_case("guard_issue_permission")

    def test_current_project_write_permission_required_before_writes(self):
        self.run_case("guard_project_permission")

    def test_current_actor_must_match_live_tpm_admission_author(self):
        self.run_case("guard_actor_mismatch")

    def test_authenticated_step_scope_cannot_add_an_unapproved_helper(self):
        self.run_case("guard_step_scope")

    def test_running_helper_source_must_equal_reviewed_immutable_source(self):
        self.run_case("guard_helper_source")

    def test_helper_closure_digest_must_match_actual_bounded_role_returns(self):
        self.run_case("guard_helper_closure_digest")

    def test_historical_raw_journal_digest_is_separately_verified(self):
        self.run_case("guard_old_raw_journal_hash")

    def test_current_raw_journal_digest_is_separately_verified(self):
        self.run_case("guard_current_raw_journal_hash")

    def test_actual_local_role_return_digest_is_verified(self):
        self.run_case("guard_role_return_digest")

    def test_closed_issue_rejected_before_metadata_writes(self):
        self.run_case("guard_issue_closed")

    def test_closed_pr_rejected_before_metadata_writes(self):
        self.run_case("guard_pr_closed")

    def test_merged_pr_rejected_before_metadata_writes(self):
        self.run_case("guard_pr_merged")

    def test_raw_hash_equal_observed_pr_payload_must_be_exact(self):
        self.run_case("guard_observed_payload_raw_hash_equal")

    def test_raw_hash_equal_reciprocal_binding_payload_must_be_exact(self):
        self.run_case("guard_binding_shape_raw_hash_equal")

    def test_live_scope_comment_drift_rejected_before_metadata_writes(self):
        self.run_case("guard_scope_comment_drift")

    def test_recovery_scope_drift_after_admission_rejected_before_writes(self):
        self.run_case("guard_recovery_context_drift")

    def test_noncanonical_observed_journal_rejected_before_writes(self):
        self.run_case("guard_noncanonical_observed_journal")

    def test_raw_hash_equal_current_action_requires_allowed_phase_and_disposition(self):
        for case in ("guard_invalid_global_phase_raw_hash_equal",
                     "guard_invalid_global_disposition_raw_hash_equal"):
            with self.subTest(case=case):
                self.run_case(case)


class PublicationMatrixTests(unittest.TestCase):
    def test_gh_comment_child_retains_publication_lock_after_parent_death(self):
        if os.name == "nt":
            self.skipTest("POSIX inherited file descriptors are required for this lease regression")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scripts = ROOT.resolve()
            common = root / "common"
            fake_bin = root / "bin"
            fake_bin.mkdir()
            pid_file = root / "gh-child.pid"
            release_file = root / "gh-child.release"
            done_file = root / "gh-child.done"
            acquired_file = root / "lock-acquired"
            fake_gh = fake_bin / "gh"
            fake_gh.write_text(
                "#!/usr/bin/env python3\n"
                "import os, time\n"
                "from pathlib import Path\n"
                "Path(os.environ['GH_CHILD_PID']).write_text(str(os.getpid()))\n"
                "while not Path(os.environ['GH_CHILD_RELEASE']).exists(): time.sleep(.02)\n"
                "Path(os.environ['GH_CHILD_DONE']).write_text('done')\n",
                encoding="utf-8",
            )
            fake_gh.chmod(0o755)
            publication_id = "sha256:" + "a" * 64
            base_env = {
                **os.environ,
                "PATH": f"{fake_bin}:{os.environ.get('PATH', '')}",
                "GH_CHILD_PID": str(pid_file),
                "GH_CHILD_RELEASE": str(release_file),
                "GH_CHILD_DONE": str(done_file),
            }
            publisher_code = "\n".join([
                "import sys, argparse",
                "from pathlib import Path",
                "sys.path.insert(0, sys.argv[1])",
                "import pr_projection_journal as j, pr_projection_publish as p",
                "common = Path(sys.argv[2])",
                "journal = j.open_journal(common, 'eng-cc/oasis7', 'feature/lease', "
                f"'{publication_id}', task_uid='{UID}', source_head_oid='{SCOPE}', "
                f"scope_base_oid='{SCOPE}', projection_digest='{CONFIG}', "
                "canonical_worktree=common)",
                "args = argparse.Namespace(issue_number=1, task_helper=common/'helper.py', "
                "repo='eng-cc/oasis7')",
                "adapter = p.GitHubPublicationAdapter(common, args, {})",
                "with journal.locked():",
                "    adapter.reservation_fd = journal.inherited_lock_fd()",
                "    adapter.gh('issue', 'comment', '1', timeout=30.0)",
            ])
            publisher = subprocess.Popen(
                [sys.executable, "-c", publisher_code, str(scripts), str(common)],
                cwd=root, env=base_env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                text=True,
            )
            child_pid = None
            contender = None
            try:
                for _ in range(300):
                    if pid_file.exists():
                        child_pid = int(pid_file.read_text(encoding="utf-8"))
                        break
                    if publisher.poll() is not None:
                        _stdout, stderr = publisher.communicate()
                        self.fail("publication parent exited before starting fake gh: "
                                  + (stderr or "<no stderr>"))
                    time.sleep(0.02)
                self.assertIsNotNone(child_pid, "fake gh did not start")
                publisher.kill()
                publisher.wait(timeout=5)
                os.kill(child_pid, 0)

                contender_code = "\n".join([
                    "import sys",
                    "from pathlib import Path",
                    "sys.path.insert(0, sys.argv[1])",
                    "import pr_projection_journal as j",
                    "common = Path(sys.argv[2])",
                    "journal = j.open_journal(common, 'eng-cc/oasis7', 'feature/lease', "
                    f"'{publication_id}', task_uid='{UID}', source_head_oid='{SCOPE}', "
                    f"scope_base_oid='{SCOPE}', projection_digest='{CONFIG}', "
                    "canonical_worktree=common)",
                    "with journal.locked():",
                    "    Path(sys.argv[3]).touch()",
                ])
                contender = subprocess.Popen(
                    [sys.executable, "-c", contender_code, str(scripts), str(common),
                     str(acquired_file)],
                    cwd=root, env=base_env, stdout=subprocess.DEVNULL,
                    stderr=subprocess.PIPE, text=True,
                )
                time.sleep(0.35)
                self.assertFalse(
                    acquired_file.exists(),
                    "a surviving gh comment child must retain the branch reservation after its parent dies",
                )

                release_file.touch()
                for _ in range(300):
                    if acquired_file.exists():
                        break
                    if contender.poll() is not None:
                        self.fail("lock contender exited before acquiring the released reservation")
                    time.sleep(0.02)
                self.assertTrue(done_file.exists(), "fake gh did not finish after release")
                self.assertTrue(acquired_file.exists(), "lock was not released after gh exited")
            finally:
                release_file.touch()
                if contender is not None and contender.poll() is None:
                    try:
                        contender.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        contender.kill()
                        contender.wait(timeout=5)
                if contender is not None:
                    contender.stderr.close()
                if publisher.poll() is not None and publisher.stderr is not None:
                    publisher.stderr.close()
                if child_pid is not None:
                    for _ in range(100):
                        try:
                            os.kill(child_pid, 0)
                        except ProcessLookupError:
                            break
                        time.sleep(0.02)
                    else:
                        try:
                            os.kill(child_pid, 9)
                        except ProcessLookupError:
                            pass

    def read_live_task_binding(self, body, repository="eng-cc/oasis7"):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            publication, _projection = make_publication(7010, repository=repository)
            args = type("Args", (), {
                "repo": repository, "issue_number": 123, "task_uid": UID,
                "task_helper": str(root / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            issue = {"number": 123, "state": "open", "body": body}
            with patch.object(adapter, "gh", return_value=json.dumps(issue)):
                result = adapter.read_task_pr_binding(UID)
            return result, adapter.pr_number

    def journal(self, temp, publication):
        return journal_module.open_journal(
            temp, publication["repository"], publication["source_ref"],
            publication["publication_id"], task_uid=publication["task_uid"],
            source_head_oid=publication["source_head_oid"],
            scope_base_oid=publication["source_scope_oid"],
            projection_digest=publication["projection_digest"],
        )

    def c1_resolution_inputs(self, index=9910):
        value, _projection = make_publication(index)
        comment = {
            "id": index, "body": publication_module.publication_comment(value),
            "created_at": "2026-09-30T12:00:00Z",
            "updated_at": "2026-09-30T12:00:00Z",
            "user": {"login": "task-author", "type": "User"},
            "author_association": "MEMBER",
        }
        pr = {
            "repository": value["repository"], "number": 991,
            "url": f"https://github.com/{value['repository']}/pull/991",
            "state": "open", "merged": False, "draft": True,
            "source_ref": value["source_ref"], "target_ref": value["target_ref"],
            "source_head_oid": value["source_head_oid"],
            "task_uid": value["task_uid"], "issue_number": 123,
            "created_at": "2026-09-30T12:01:00Z",
            "updated_at": "2026-09-30T12:01:00Z",
            "task_status": "committed", "task_phase": "verification",
            "task_pr_number": 991,
            "task_pr_url": f"https://github.com/{value['repository']}/pull/991",
            "pr_author": "task-author", "pr_author_type": "User",
        }
        read = {"complete": True, "repository": value["repository"],
                "issue_number": 123, "comments": [comment]}
        return value, comment, read, pr

    def test_c1_live_resolver_returns_exact_publication_and_server_provenance(self):
        value, comment, read, pr = self.c1_resolution_inputs()
        result = publication_module.resolve_task_publication(
            read, {key: value[key] for key in publication_module._TASK_PUBLICATION_FIELDS},
            live_task_author={"login": "task-author", "type": "User"},
            pr_binding=pr,
        )
        self.assertEqual("passed", result["status"])
        self.assertEqual(value, result["publication"])
        self.assertEqual(comment["id"], result["comment"]["comment_id"])
        self.assertEqual(comment["user"], result["comment"]["author"])
        self.assertEqual("MEMBER", result["comment"]["author_association"])
        self.assertEqual(991, result["pr_number"])

    def test_c1_live_resolver_fails_closed_on_incomplete_duplicate_or_wrong_pr(self):
        value, _comment, read, pr = self.c1_resolution_inputs(9911)
        expected = {key: value[key] for key in publication_module._TASK_PUBLICATION_FIELDS}
        author = {"login": "task-author", "type": "User"}
        incomplete = publication_module.resolve_task_publication(
            {**read, "complete": False}, expected, live_task_author=author, pr_binding=pr,
        )
        self.assertEqual("pending", incomplete["status"])
        duplicate = publication_module.resolve_task_publication(
            {**read, "comments": read["comments"] * 2}, expected,
            live_task_author=author, pr_binding=pr,
        )
        self.assertEqual("blocked", duplicate["status"])
        wrong_pr = publication_module.resolve_task_publication(
            read, expected, live_task_author=author,
            pr_binding={**pr, "source_head_oid": "f" * 40},
        )
        self.assertEqual("blocked", wrong_pr["status"])
        malformed = publication_module.resolve_task_publication(
            {**read, "comments": [dict(read["comments"][0], body="<!-- oasis7-ci-publication/v1 -->\n{}") ]},
            expected, live_task_author=author, pr_binding=pr,
        )
        self.assertEqual("blocked", malformed["status"])

    def test_c1_live_resolver_does_not_claim_permission_from_comment_author(self):
        value, _comment, read, pr = self.c1_resolution_inputs(9912)
        result = publication_module.resolve_task_publication(
            read, {key: value[key] for key in publication_module._TASK_PUBLICATION_FIELDS},
            live_task_author={"login": "task-author", "type": "User"},
            permissions={}, pr_binding=pr,
        )
        self.assertEqual("passed", result["status"])
        self.assertNotIn("permissions", result)

    def test_c1_live_resolver_accepts_bound_draft_update_before_update_and_rejects_late(self):
        value, comment, read, pr = self.c1_resolution_inputs(9913)
        expected = {key: value[key] for key in publication_module._TASK_PUBLICATION_FIELDS}
        author = {"login": "task-author", "type": "User"}
        initial = publication_module.resolve_task_publication(
            read, expected, live_task_author=author, pr_binding=pr,
        )
        self.assertEqual("passed", initial["status"], initial)

        mismatch = publication_module.resolve_task_publication(
            read, expected, live_task_author={"login": "other-user", "type": "User"},
            pr_binding=pr,
        )
        self.assertEqual("blocked", mismatch["status"])

        updated_draft = {**pr, "created_at": "2026-09-30T12:01:00Z",
                         "updated_at": "2026-09-30T12:03:00Z"}
        between = publication_module.resolve_task_publication(
            {**read, "comments": [dict(comment, created_at="2026-09-30T12:02:00Z",
                                        updated_at="2026-09-30T12:02:00Z")]},
            expected, live_task_author=author, pr_binding=updated_draft,
        )
        self.assertEqual("passed", between["status"], between)
        for timestamp in ("2026-09-30T12:03:00Z", "2026-09-30T12:04:00Z"):
            with self.subTest(published_at=timestamp):
                late = publication_module.resolve_task_publication(
                    {**read, "comments": [dict(comment, created_at=timestamp,
                                                updated_at=timestamp)]},
                    expected, live_task_author=author, pr_binding=updated_draft,
                )
                self.assertEqual("blocked", late["status"], late)

        wrong_phase = publication_module.resolve_task_publication(
            read, expected, live_task_author=author,
            pr_binding={**pr, "task_status": "ready", "task_phase": "pre_pr_ready"},
        )
        self.assertEqual("blocked", wrong_phase["status"])

    def test_c1_live_resolver_rejects_missing_invalid_or_edited_server_timestamps(self):
        value, comment, read, pr = self.c1_resolution_inputs(9915)
        expected = {key: value[key] for key in publication_module._TASK_PUBLICATION_FIELDS}
        author = {"login": "task-author", "type": "User"}
        cases = {
            "updated_at edited": dict(comment, updated_at="2026-09-30T12:00:01Z"),
            "updated_at missing": {key: value for key, value in comment.items() if key != "updated_at"},
            "updated_at invalid": dict(comment, updated_at="not-a-server-timestamp"),
            "created_at missing": {key: value for key, value in comment.items() if key != "created_at"},
            "created_at invalid": dict(comment, created_at="not-a-server-timestamp"),
        }
        for label, candidate in cases.items():
            with self.subTest(timestamp_case=label):
                result = publication_module.resolve_task_publication(
                    {**read, "comments": [candidate]}, expected,
                    live_task_author=author, pr_binding=pr,
                )
                self.assertIn(result["status"], {"blocked", "pending"}, result)

    def test_c1_start_window_allows_only_both_task_pr_binding_fields_absent(self):
        value, _comment, read, pr = self.c1_resolution_inputs(9914)
        expected = {key: value[key] for key in publication_module._TASK_PUBLICATION_FIELDS}
        issue_unbound = {**pr, "task_status": "committed", "task_phase": "execution",
                         "task_pr_number": None, "task_pr_url": None}
        accepted = publication_module.resolve_task_publication(
            read, expected, live_task_author={"login": "task-author", "type": "User"},
            pr_binding=issue_unbound,
        )
        self.assertEqual("passed", accepted["status"], accepted)
        one_sided = publication_module.resolve_task_publication(
            read, expected, live_task_author={"login": "task-author", "type": "User"},
            pr_binding={**issue_unbound, "task_pr_number": 991},
        )
        self.assertEqual("blocked", one_sided["status"])
        wrong_pr_author = publication_module.resolve_task_publication(
            read, expected, live_task_author={"login": "task-author", "type": "User"},
            pr_binding={**pr, "pr_author": "another-user"},
        )
        self.assertEqual("blocked", wrong_pr_author["status"])

    def test_branch_journal_lock_handoff_allows_child_writer_and_reacquires_parent(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, _projection = make_publication(9915)
            journal = self.journal(temp, publication)
            with journal.locked():
                journal.intent("record-pr:" + publication["publication_id"], "record_pr", {
                    "publication_id": publication["publication_id"],
                    "task_uid": publication["task_uid"], "pr_number": 991,
                })
                code = (
                    "import sys; from pathlib import Path; "
                    "sys.path.insert(0, sys.argv[1]); import pr_projection_journal as j; "
                    "p=j.open_journal(sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], "
                    "task_uid=sys.argv[6], source_head_oid=sys.argv[7], "
                    "scope_base_oid=sys.argv[8], projection_digest=sys.argv[9]); "
                    "\nwith p.locked():\n "
                    "p.intent('child-step','record_pr_issue',{'task_uid':sys.argv[6]})\n "
                    "p.observe('child-step',{'issue':'target'})\n"
                )
                with journal.release_for_child_writer():
                    child = subprocess.run([
                        sys.executable, "-c", code, str(Path(__file__).parent), temp,
                        publication["repository"], publication["source_ref"],
                        publication["publication_id"], publication["task_uid"],
                        publication["source_head_oid"], publication["source_scope_oid"],
                        publication["projection_digest"],
                    ], capture_output=True, text=True, timeout=5)
                    self.assertEqual(0, child.returncode, child.stderr)
                actions = journal.read()["actions"]
                self.assertEqual(["record-pr:" + publication["publication_id"], "child-step"],
                                 [item["action_id"] for item in actions])
                self.assertEqual("observed", actions[1]["state"])

    def run_publish_entrypoint(self, temp, publication, projection, adapter, journal,
                               *, existing_ready_update=False, body=None,
                               resume_action_id=None):
        root = Path(temp)
        body_file = root / "body.md"
        body_file.write_text(body if body is not None else f"Task: {UID}\nRefs #1\n", encoding="utf-8")
        projection_file = root / "projection.json"
        projection_file.write_text("{}\n", encoding="utf-8")
        args_values = {
            "worktree": str(root), "task_uid": UID, "issue_number": 1,
            "body_file": str(body_file), "projection": str(projection_file),
            "existing_ready_update": existing_ready_update,
        }
        if resume_action_id is not None:
            args_values["resume_action_id"] = resume_action_id
        args = type("Args", (), args_values)()
        with (
            patch.object(publish_module, "task_publication", return_value=(publication, projection)),
            patch.object(publish_module, "GitHubPublicationAdapter", return_value=adapter),
            patch.object(publish_module, "repo_common_dir", return_value=root),
            patch.object(publish_module.pr_projection_journal, "open_journal", return_value=journal),
        ):
            return publish_module.publish(args)

    def existing_update_case(self, temp, index, *, old_head, new_head,
                             adapter_type=FakeAdapter):
        publication, projection = make_publication(index, head=new_head)
        pr = {
            "repository": publication["repository"], "source_ref": publication["source_ref"],
            "target_ref": publication["target_ref"], "head_oid": old_head,
            "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
            "draft": True, "number": index,
        }
        adapter = adapter_type(
            publication, projection, initial_head=old_head, initial_pr=pr,
        )
        adapter.pr_binding = None
        journal = self.journal(temp, publication)
        return publication, projection, adapter, journal

    def assert_old_head_rejected_before_effects(self, attempt, adapter, journal,
                                                *, prior_action_ids=()):
        error = None
        result = None
        try:
            result = attempt()
        except Exception as exc:  # capture the real failure signature for this boundary
            error = exc
        with journal.locked():
            actions = journal.read()["actions"]
        action_ids = [item.get("action_id") for item in actions]
        action_kinds = [item.get("kind") for item in actions]
        writes = {
            event: adapter.events.count(event)
            for event in ("task-intent", "patch-pr", "push", "record-pr", "publish-reciprocal")
        }
        evidence = (
            f"result={result!r} error={type(error).__name__ if error else None}: {error!r}; "
            f"events={adapter.events!r}; journal_action_ids={action_ids!r}; "
            f"journal_action_kinds={action_kinds!r}; write_counts={writes!r}"
        )
        self.assertIsInstance(
            error, (publication_module.PublicationError, publish_module.PublishInputError),
            "old H0 must fail closed before publication effects; " + evidence,
        )
        self.assertEqual(list(prior_action_ids), action_ids, evidence)
        self.assertEqual({
            "task-intent": 0, "patch-pr": 0, "push": 0,
            "record-pr": 0, "publish-reciprocal": 0,
        }, writes, evidence)

    def test_invalid_caller_discovered_and_readback_h0_rejected_before_intent(self):
        """Reject malformed H0 even when fake discovery and live readback agree."""
        invalid_heads = (None, "", "not-an-oid", "a" * 39, "A" * 40)
        for source in ("caller", "discovered-readback"):
            for case, old_head in enumerate(invalid_heads):
                with self.subTest(source=source, old_head=old_head):
                    with tempfile.TemporaryDirectory() as temp:
                        publication, projection = make_publication(1110 + case, head="d" * 40)
                        pr_number = 201 + case
                        body = f"Task: {UID}\nRefs #1\n"
                        pr = {
                            "repository": publication["repository"],
                            "source_ref": publication["source_ref"],
                            "target_ref": publication["target_ref"],
                            "head_oid": old_head, "body": body,
                            "state": "open", "merged": False, "draft": True,
                            "number": pr_number,
                        }
                        adapter = FakeAdapter(
                            publication, projection, initial_head=old_head, initial_pr=pr,
                        )
                        journal = self.journal(temp, publication)

                        if source == "caller":
                            attempt = lambda: publication_module.publish_update(
                                adapter, journal, publication=publication,
                                projection=projection, pr_number=pr_number,
                                old_head_oid=old_head, body=body,
                            )
                        else:
                            attempt = lambda: self.run_publish_entrypoint(
                                temp, publication, projection, adapter, journal,
                            )
                        self.assert_old_head_rejected_before_effects(attempt, adapter, journal)

    def test_null_discovered_head_and_absent_source_ref_rejected_before_intent(self):
        """A missing PR head plus absent source ref must never acquire a null lease."""
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(1118, head="e" * 40)
            pr = {
                "repository": publication["repository"],
                "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"],
                "head_oid": None, "body": f"Task: {UID}\nRefs #1\n",
                "state": "open", "merged": False, "draft": True, "number": 219,
            }
            adapter = FakeAdapter(publication, projection, initial_head=None, initial_pr=pr)
            journal = self.journal(temp, publication)
            self.assertIsNone(adapter.source_ref, "fixture must model an absent source ref")
            self.assert_old_head_rejected_before_effects(
                lambda: self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                ),
                adapter, journal,
            )

    def test_malformed_journal_pinned_h0_rejected_before_task_or_pr_writes(self):
        invalid_heads = (None, "", "not-an-oid", "b" * 39, "B" * 40)
        for case, pinned_head in enumerate(invalid_heads):
            with self.subTest(pinned_head=pinned_head):
                with tempfile.TemporaryDirectory() as temp:
                    publication, projection = make_publication(1120 + case, head="d" * 40)
                    pr_number = 230 + case
                    body = f"Task: {UID}\nRefs #1\n"
                    pr = {
                        "repository": publication["repository"],
                        "source_ref": publication["source_ref"],
                        "target_ref": publication["target_ref"],
                        "head_oid": pinned_head, "body": body,
                        "state": "open", "merged": False, "draft": True,
                        "number": pr_number,
                    }
                    adapter = FakeAdapter(
                        publication, projection, initial_head=pinned_head, initial_pr=pr,
                    )
                    journal = self.journal(temp, publication)
                    state_id = "update-state:" + publication["publication_id"]
                    with journal.locked():
                        journal.intent(state_id, "pin_update_state", {
                            "pr_number": pr_number, "expected_draft": True,
                            "existing_ready_update": False, "old_head_oid": pinned_head,
                        })
                    self.assert_old_head_rejected_before_effects(
                        lambda: publication_module.publish_update(
                            adapter, journal, publication=publication,
                            projection=projection, pr_number=pr_number,
                            old_head_oid="c" * 40, body=body,
                        ),
                        adapter, journal, prior_action_ids=(state_id,),
                    )

    def test_malformed_recovery_patch_h0_rejected_before_new_intents_or_writes(self):
        invalid_heads = (None, "", "not-an-oid", "c" * 39, "C" * 40)
        for case, recovery_head in enumerate(invalid_heads):
            with self.subTest(recovery_head=recovery_head):
                with tempfile.TemporaryDirectory() as temp:
                    publication, projection = make_publication(1130 + case, head="e" * 40)
                    pr_number = 240 + case
                    body = publication_module.replace_projection_marker(
                        f"Task: {UID}\nRefs #1\n",
                        publication_module.prepare(
                            task_uid=UID,
                            source_head_oid=publication["source_head_oid"],
                            scope_base_oid=publication["source_scope_oid"],
                            projection_digest=publication["projection_digest"],
                        )[1],
                    )
                    pr = {
                        "repository": publication["repository"],
                        "source_ref": publication["source_ref"],
                        "target_ref": publication["target_ref"],
                        "head_oid": publication["source_head_oid"], "body": body,
                        "state": "open", "merged": False, "draft": True,
                        "number": pr_number,
                    }
                    adapter = FakeAdapter(
                        publication, projection,
                        initial_head=publication["source_head_oid"], initial_pr=pr,
                    )
                    journal = self.journal(temp, publication)
                    patch_id = "patch-body:" + publication["publication_id"]
                    body_hash = "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest()
                    with journal.locked():
                        journal.intent(patch_id, "patch_projection_body", {
                            "pr_number": pr_number, "new_body_sha256": body_hash,
                            "old_head_oid": recovery_head,
                        })
                    self.assert_old_head_rejected_before_effects(
                        lambda: publication_module.publish_update(
                            adapter, journal, publication=publication,
                            projection=projection, pr_number=pr_number,
                            old_head_oid="b" * 40, body=body,
                        ),
                        adapter, journal, prior_action_ids=(patch_id,),
                    )

    def test_record_pr_passes_canonical_repository_to_task_helper(self):
        publication, _projection = make_publication(7000, repository="example/oasis7")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": "example/oasis7", "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            with patch.object(publish_module, "command_output", return_value="") as command:
                adapter.record_pr(UID, 999, publication["publication_id"])

        self.assertEqual(60, command.call_args.kwargs["timeout"])
        argv = command.call_args.args[0]
        self.assertIn("record-pr", argv)
        self.assertIn("--repo", argv)
        repo_index = argv.index("--repo")
        self.assertEqual("example/oasis7", argv[repo_index + 1])
        self.assertEqual("https://github.com/example/oasis7/pull/999",
                         argv[argv.index("--pr-url") + 1])

    def test_record_pr_forwards_explicit_ready_update_mode(self):
        publication, _projection = make_publication(7000)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
                "existing_ready_update": True,
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            with patch.object(publish_module, "command_output", return_value="") as command:
                adapter.record_pr(UID, 999, publication["publication_id"])

        argv = command.call_args.args[0]
        self.assertIn("--existing-ready-update", argv)
        self.assertNotIn("--draft-candidate", argv)

    def test_target_oid_cannot_be_substituted_for_projection_source_scope(self):
        publication, projection = make_publication(7001)
        target_oid = "e" * 40
        self.assertNotEqual(publication["source_scope_oid"], target_oid)
        projection["scope_base_oid"] = target_oid
        with tempfile.TemporaryDirectory() as temp:
            adapter = FakeAdapter(publication, projection)
            journal = self.journal(temp, publication)
            with self.assertRaisesRegex(
                publication_module.PublicationError, "impact projection scope_base_oid",
            ):
                publication_module.publish_create(
                    adapter, journal, publication=publication,
                    projection=projection, body="Task: " + UID + "\nRefs #1",
                )
            self.assertEqual([], adapter.events)
            with journal.locked():
                self.assertEqual([], journal.read()["actions"])

    def test_task_binding_readback_accepts_valid_and_absent_pr_url(self):
        valid, remembered_number = self.read_live_task_binding(
            f"task_uid: {UID}\n- pr_url: `https://github.com/eng-cc/oasis7/pull/3989`\n",
        )
        self.assertEqual({"task_uid": UID, "pr_number": 3989}, valid)
        self.assertEqual(3989, remembered_number)

        absent, remembered_number = self.read_live_task_binding(f"task_uid: {UID}\n")
        self.assertEqual({"task_uid": UID, "pr_number": None}, absent)
        self.assertIsNone(remembered_number)

    def test_production_ready_admission_marker_requires_matching_task_and_nondraft_pr(self):
        publication, _projection = make_publication(7005)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            pr_number = 115
            pr_url = f"https://github.com/{publication['repository']}/pull/{pr_number}"
            record = {
                "task_uid": UID,
                "repository": publication["repository"],
                "issue_number": 123,
                "task_branch": publication["source_ref"],
                "default_branch": publication["target_ref"],
                "canonical_worktree": str(root),
                "status": "pr_watch",
                "workflow_phase": "pr_watch",
                "pr_number": pr_number,
                "pr_url": pr_url,
            }
            mapping_path = root / ".pm/github-project-sync/tasks.json"
            mapping_path.parent.mkdir(parents=True)
            mapping_path.write_text(json.dumps({"version": 1, "tasks": {UID: record}}), encoding="utf-8")
            issue = {
                "number": 123,
                "state": "open",
                "body": "\n".join((
                    f"task_uid: {UID}",
                    "- status: `pr_watch`",
                    "- workflow_phase: `pr_watch`",
                    f"- worktree_hint: `{root}`",
                    f"- pr_url: `{pr_url}`",
                    f"- pr_number: `{pr_number}`",
                )),
            }
            live_pr = {
                "number": pr_number,
                "html_url": pr_url,
                "state": "open",
                "merged_at": None,
                "draft": False,
                "head": {
                    "ref": publication["source_ref"],
                    "sha": publication["source_head_oid"],
                    "repo": {"full_name": publication["repository"]},
                },
                "base": {
                    "ref": publication["target_ref"],
                    "repo": {"full_name": publication["repository"]},
                },
                "body": f"Task: {UID}\nRefs #123\n",
            }
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
                "source_ref": publication["source_ref"], "target_ref": publication["target_ref"],
                "existing_ready_update": True,
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)

            def fake_gh(*command, timeout=5.0, input_json=None):
                if command == ("api", f"repos/{publication['repository']}/issues/123"):
                    return json.dumps(issue)
                if command == ("api", f"repos/{publication['repository']}/pulls/{pr_number}"):
                    return json.dumps(live_pr)
                raise AssertionError(f"unexpected live-adapter read: {command}")

            with patch.object(adapter, "gh", side_effect=fake_gh):
                admitted = adapter.read_task_pr_binding(UID)
            self.assertEqual({
                "task_uid": UID,
                "pr_number": pr_number,
                "existing_ready_update": True,
            }, admitted)

            live_pr["draft"] = True
            with patch.object(adapter, "gh", side_effect=fake_gh):
                with self.assertRaisesRegex(RuntimeError, "live PR repository/ref/head/state identity mismatch"):
                    adapter.read_task_pr_binding(UID)

    def test_ready_update_rejects_noncanonical_task_and_refs_lines_at_each_body_boundary(self):
        wrong_uid = "task_" + "b" * 32
        malformed_bodies = {
            "wrong_task_uid": f"Task: {wrong_uid}\nRefs #1\n",
            "task_uid_suffix": f"Task: {UID}-stale\nRefs #1\n",
            "wrong_issue_number": f"Task: {UID}\nRefs #2\n",
            "refs_number_suffix": f"Task: {UID}\nRefs #1-suffix\n",
            "missing_task_line_with_prose_substring": f"prose says Task: {UID}\nRefs #1\n",
            "missing_refs_line_with_prose_substring": f"Task: {UID}\nprose says Refs #1\n",
            "duplicate_task_same_value": f"Task: {UID}\nTask: {UID}\nRefs #1\n",
            "duplicate_task_conflicting_value": f"Task: {UID}\nTask: {wrong_uid}\nRefs #1\n",
            "duplicate_refs_same_value": f"Task: {UID}\nRefs #1\nRefs #1\n",
            "duplicate_refs_conflicting_value": f"Task: {UID}\nRefs #1\nRefs #2\n",
        }
        write_events = {"task-intent", "patch-pr", "push", "record-pr", "publish-reciprocal"}

        for boundary in ("supplied", "discovered"):
            for name, malformed_body in malformed_bodies.items():
                with self.subTest(boundary=boundary, body=name), tempfile.TemporaryDirectory() as temp:
                    old_head = "c" * 40
                    new_head = "d" * 40
                    publication, projection = make_publication(1200 + len(name), head=new_head)
                    pr = {
                        "repository": publication["repository"],
                        "source_ref": publication["source_ref"],
                        "target_ref": publication["target_ref"],
                        "head_oid": old_head,
                        "body": malformed_body if boundary == "discovered" else f"Task: {UID}\nRefs #1\n",
                        "state": "open", "merged": False, "draft": False, "number": 120,
                    }
                    adapter = FakeAdapter(
                        publication, projection, initial_head=old_head, initial_pr=pr,
                    )
                    adapter.ready_update_admitted = True
                    journal = self.journal(temp, publication)
                    supplied_body = malformed_body if boundary == "supplied" else f"Task: {UID}\nRefs #1\n"

                    error = None
                    try:
                        self.run_publish_entrypoint(
                            temp, publication, projection, adapter, journal,
                            existing_ready_update=True, body=supplied_body,
                        )
                    except (publish_module.PublishInputError, publication_module.PublicationError) as exc:
                        error = exc

                    leaked = [event for event in adapter.events if event in write_events]
                    self.assertIsNotNone(
                        error,
                        f"{boundary} body {name} was accepted; result events={adapter.events!r}",
                    )
                    self.assertEqual(
                        [], leaked,
                        f"{boundary} body {name} reached C1 writes before rejection: {adapter.events!r}",
                    )

    def test_ready_update_rejects_noncanonical_live_pr_task_and_refs_lines_before_writes(self):
        wrong_uid = "task_" + "b" * 32
        malformed_bodies = {
            "wrong_task_uid": f"Task: {wrong_uid}\nRefs #123\n",
            "task_uid_suffix": f"Task: {UID}-stale\nRefs #123\n",
            "wrong_issue_number": f"Task: {UID}\nRefs #124\n",
            "refs_number_suffix": f"Task: {UID}\nRefs #1234\n",
            "missing_task_line_with_prose_substring": f"prose says Task: {UID}\nRefs #123\n",
            "missing_refs_line_with_prose_substring": f"Task: {UID}\nprose says Refs #123\n",
            "duplicate_task_same_value": f"Task: {UID}\nTask: {UID}\nRefs #123\n",
            "duplicate_task_conflicting_value": f"Task: {UID}\nTask: {wrong_uid}\nRefs #123\n",
            "duplicate_refs_same_value": f"Task: {UID}\nRefs #123\nRefs #123\n",
            "duplicate_refs_conflicting_value": f"Task: {UID}\nRefs #123\nRefs #124\n",
        }

        for name, body in malformed_bodies.items():
            with self.subTest(body=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp).resolve()
                publication, _projection = make_publication(1300 + len(name))
                source_ref = publication["source_ref"]
                target_ref = publication["target_ref"]
                record = {
                    "task_uid": UID,
                    "repository": publication["repository"],
                    "issue_number": 123,
                    "task_branch": source_ref,
                    "default_branch": target_ref,
                    "canonical_worktree": str(root),
                    "worktree_hint": str(root),
                    "status": "pr_watch",
                    "workflow_phase": "pr_watch",
                    "pr_number": 123,
                    "pr_url": f"https://github.com/{publication['repository']}/pull/123",
                }
                mapping_path = root / ".pm/github-project-sync/tasks.json"
                mapping_path.parent.mkdir(parents=True)
                mapping_path.write_text(json.dumps({"version": 1, "tasks": {UID: record}}), encoding="utf-8")
                args = type("Args", (), {
                    "repo": publication["repository"], "issue_number": 123,
                    "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
                    "source_ref": source_ref, "target_ref": target_ref,
                    "existing_ready_update": True,
                })()
                issue = {
                    "number": 123,
                    "state": "open",
                    "body": (
                        f"task_uid: {UID}\n- status: `pr_watch`\n- workflow_phase: `pr_watch`\n"
                        f"- worktree_hint: `{root}`\n- pr_url: `{record['pr_url']}`\n- pr_number: `123`\n"
                    ),
                }
                live_pr = {
                    "number": 123,
                    "html_url": record["pr_url"],
                    "state": "open",
                    "merged_at": None,
                    "draft": False,
                    "head": {"ref": source_ref, "sha": publication["source_head_oid"],
                             "repo": {"full_name": publication["repository"]}},
                    "base": {"ref": target_ref, "repo": {"full_name": publication["repository"]}},
                    "body": body,
                }
                adapter = publish_module.GitHubPublicationAdapter(root, args, publication)

                def fake_gh(*command, timeout=5.0):
                    if command == ("api", f"repos/{publication['repository']}/issues/123"):
                        return json.dumps(issue)
                    if command == ("api", f"repos/{publication['repository']}/pulls/123"):
                        return json.dumps(live_pr)
                    raise AssertionError(f"unexpected adapter read: {command}")

                with patch.object(adapter, "gh", side_effect=fake_gh):
                    try:
                        adapter.read_task_pr_binding(UID)
                    except RuntimeError:
                        error = True
                    else:
                        error = False
                self.assertTrue(error, f"live PR body {name} was accepted as reciprocal identity")

    def test_task_binding_readback_rejects_wrong_repo_and_malformed_identity(self):
        with self.assertRaisesRegex(publication_module.ContractError, "repository mismatch"):
            self.read_live_task_binding(
                f"task_uid: {UID}\n- pr_url: `https://github.com/other/oasis7/pull/3989`\n",
            )
        with self.assertRaisesRegex(publication_module.ContractError, "PR URL is malformed"):
            self.read_live_task_binding(
                f"task_uid: {UID}\n- pr_url: `https://github.com/eng-cc/oasis7/pull/not-a-number`\n",
            )
        with self.assertRaisesRegex(RuntimeError, "Task issue PR binding is malformed"):
            self.read_live_task_binding(
                f"task_uid: {UID}\n- pr_url: https://github.com/eng-cc/oasis7/pull/3989\n",
            )
        with self.assertRaisesRegex(RuntimeError, "Task issue PR binding is ambiguous"):
            self.read_live_task_binding(
                f"task_uid: {UID}\n"
                "- pr_url: `https://github.com/eng-cc/oasis7/pull/3989`\n"
                "- pr_url: `https://github.com/eng-cc/oasis7/pull/3990`\n",
            )
        with self.assertRaisesRegex(RuntimeError, "canonical Task issue UID mismatch"):
            self.read_live_task_binding(
                f"task_uid: {'task_' + 'b' * 32}\n"
                "- pr_url: `https://github.com/eng-cc/oasis7/pull/3989`\n",
            )

    def test_issue_comments_paginates_100_items_and_rejects_incomplete_pages(self):
        publication, _projection = make_publication(7005)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            first_page = [
                {"id": 10, "body": "older task evidence"},
                {"id": 11, "body": "older task follow-up"},
            ]
            second_page = [{"id": 110, "body": "latest task decision"}]
            with patch.object(
                adapter, "gh", return_value=json.dumps([first_page, second_page]),
            ) as gh:
                comments = adapter._issue_comments()

            self.assertEqual(first_page + second_page, comments)
            gh.assert_called_once()
            request_args = gh.call_args.args
            self.assertEqual("api", request_args[0])
            self.assertIn("repos/eng-cc/oasis7/issues/123/comments", request_args[1])
            self.assertIn("--paginate", request_args)
            self.assertIn("--slurp", request_args)
            request_timeout = gh.call_args.kwargs["timeout"]
            requests_100_comments = any(
                "per_page=100" in str(argument) for argument in request_args
            )

            malformed_responses = {
                "non-array second page": json.dumps([
                    first_page, {"id": 110, "body": "not a slurped page"},
                ]),
                "missing comment body on later page": json.dumps([
                    first_page, [{"id": 110}],
                ]),
                "malformed JSON": "[{not-json]",
            }
            for label, response in malformed_responses.items():
                with self.subTest(response=label):
                    with patch.object(adapter, "gh", return_value=response):
                        with self.assertRaisesRegex(RuntimeError, "malformed"):
                            adapter._issue_comments()

            self.assertTrue(
                requests_100_comments,
                "Task comments pagination must request per_page=100",
            )
            self.assertEqual(
                30.0, request_timeout,
                "complete Task comments pagination must use the 30-second read budget",
            )

    def test_publish_task_intent_requires_exact_full_body_readback(self):
        publication, _projection = make_publication(7006)
        expected_body = publication_module.publication_comment(publication)
        issue = {"number": 123, "state": "open", "body": f"task_uid: {UID}\n"}

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            adapter.authenticated_login = "publisher"

            def exact_readback(*command, timeout=5.0, input_json=None):
                if command[:2] == ("api", f"repos/{publication['repository']}/issues/123"):
                    return json.dumps(issue)
                if command[:2] == ("issue", "comment"):
                    body_path = Path(command[command.index("--body-file") + 1])
                    self.assertEqual(expected_body, body_path.read_text(encoding="utf-8"))
                    return f"https://github.com/{publication['repository']}/issues/123#issuecomment-7006"
                if command[:2] == (
                    "api", f"repos/{publication['repository']}/issues/comments/7006",
                ):
                    return json.dumps({
                        "id": 7006, "body": expected_body,
                        "created_at": "2026-09-30T12:00:00Z",
                        "updated_at": "2026-09-30T12:00:00Z",
                        "user": {"login": "publisher"},
                    })
                raise AssertionError(f"unexpected mocked GitHub call: {command!r}")

            with patch.object(adapter, "gh", side_effect=exact_readback):
                adapter.publish_task_intent(publication)

    def test_prepublication_read_timeout_creates_no_task_write_intent(self):
        old_head = "8" * 40
        new_head = "9" * 40
        publication, projection = make_publication(7190, head=new_head)
        pr = {
            "repository": publication["repository"], "source_ref": publication["source_ref"],
            "target_ref": publication["target_ref"], "head_oid": old_head,
            "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
            "draft": True, "number": 190,
        }

        with tempfile.TemporaryDirectory() as temp:
            adapter = FirstPublicationReadTimeoutAdapter(
                publication, projection, initial_head=old_head, initial_pr=pr,
            )
            journal = self.journal(temp, publication)
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task publication readback failed",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)

            action_id = "task-intent:" + publication["publication_id"]
            with journal.locked():
                state = journal.read()
            encoded_state = json.dumps(state, sort_keys=True)
            task_actions = [
                action for action in state.get("actions", [])
                if action.get("action_id") == action_id
            ]
            pending_task_writes = [
                action for action in task_actions
                if action.get("state") in {"intent", "uncertain"}
            ]
            self.assertEqual(1, adapter.publication_reads)
            self.assertEqual(0, adapter.events.count("task-intent"))
            self.assertNotIn("patch-pr", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn('"WRITE_INTENT"', encoded_state)
            self.assertNotIn('"POST_ATTEMPTED"', encoded_state)
            self.assertEqual(
                [], pending_task_writes,
                "a failed prepublication read may leave read evidence, never a pending write intent",
            )

    def test_denied_or_incomplete_prepublication_read_never_posts(self):
        class PreReadFailureAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.failure_kind = "denied"

            def find_task_publications(inner_self, publication_id):
                inner_self.events.append("read-task-publications")
                if inner_self.failure_kind == "denied":
                    raise PermissionError("simulated comments-read denial")
                return {"complete": False, "publications": []}

        old_head, new_head = "9" * 40, "a" * 40
        for failure_kind in ("denied", "incomplete"):
            with self.subTest(failure_kind=failure_kind), tempfile.TemporaryDirectory() as temp:
                publication, projection, adapter, journal = self.existing_update_case(
                    temp, 7220 if failure_kind == "denied" else 7221,
                    old_head=old_head, new_head=new_head,
                    adapter_type=PreReadFailureAdapter,
                )
                adapter.failure_kind = failure_kind
                with self.assertRaises(publication_module.PublicationError):
                    self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
                action_id = "task-intent:" + publication["publication_id"]
                with journal.locked():
                    events = journal.read_task_events(action_id)
                self.assertEqual("READ_FAILED", events[-1]["event"])
                self.assertNotIn("WRITE_INTENT", [item["event"] for item in events])
                self.assertNotIn("POST_ATTEMPTED", [item["event"] for item in events])
                self.assertEqual(0, adapter.events.count("task-intent"))

    def test_same_action_recovers_from_prepublication_read_timeout(self):
        old_head = "a" * 40
        new_head = "b" * 40
        publication, projection = make_publication(7191, head=new_head)
        pr = {
            "repository": publication["repository"], "source_ref": publication["source_ref"],
            "target_ref": publication["target_ref"], "head_oid": old_head,
            "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
            "draft": True, "number": 191,
        }

        with tempfile.TemporaryDirectory() as temp:
            adapter = FirstPublicationReadTimeoutAdapter(
                publication, projection, initial_head=old_head, initial_pr=pr,
            )
            journal = self.journal(temp, publication)
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task publication readback failed",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)

            action_id = "task-intent:" + publication["publication_id"]
            with journal.locked():
                before_recovery = journal.read()
            self.assertIn(action_id, json.dumps(before_recovery, sort_keys=True))
            original_event_bytes = journal.task_events_path.read_bytes()

            adapter.events.clear()
            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
                resume_action_id=action_id,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(1, adapter.events.count("task-intent"))
            self.assertEqual(3, adapter.publication_reads)
            self.assertEqual(1, len(adapter.publications))
            with journal.locked():
                after_recovery = journal.read()
            self.assertIn(action_id, json.dumps(after_recovery, sort_keys=True))
            self.assertTrue(
                journal.task_events_path.read_bytes().startswith(original_event_bytes),
                "recovery must append evidence without rewriting prior event history",
            )

    def test_post_attempt_marker_is_durable_before_adapter_invocation(self):
        old_head = "c" * 40
        new_head = "d" * 40
        publication, projection = make_publication(7192, head=new_head)
        pr = {
            "repository": publication["repository"], "source_ref": publication["source_ref"],
            "target_ref": publication["target_ref"], "head_oid": old_head,
            "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
            "draft": True, "number": 192,
        }

        with tempfile.TemporaryDirectory() as temp:
            journal = self.journal(temp, publication)

            class MarkerCheckingAdapter(FakeAdapter):
                def publish_task_intent(inner_self, value):
                    events = journal.read().get("task_post_events", [])
                    action_id = "task-intent:" + publication["publication_id"]
                    action_events = [
                        event for event in events if event.get("action_id") == action_id
                    ]
                    self.assertGreaterEqual(len(action_events), 2)
                    self.assertEqual("WRITE_INTENT", action_events[-2].get("event"))
                    self.assertEqual("POST_ATTEMPTED", action_events[-1].get("event"))
                    super(MarkerCheckingAdapter, inner_self).publish_task_intent(value)

            adapter = MarkerCheckingAdapter(
                publication, projection, initial_head=old_head, initial_pr=pr,
            )
            adapter.pr_binding = None
            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(1, adapter.events.count("task-intent"))

    def test_write_intent_without_attempt_marker_allows_one_recovery_post(self):
        old_head, new_head = "e" * 40, "f" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7193, old_head=old_head, new_head=new_head,
            )
            action_id = "task-intent:" + publication["publication_id"]
            append_event = journal.append_task_event

            def crash_after_durable_write_intent(action, event, identity, evidence=None):
                record = append_event(action, event, identity, evidence)
                if event == "WRITE_INTENT":
                    raise RuntimeError("simulated crash before POST_ATTEMPTED")
                return record

            with patch.object(journal, "append_task_event", side_effect=crash_after_durable_write_intent):
                with self.assertRaisesRegex(RuntimeError, "simulated crash"):
                    self.run_publish_entrypoint(temp, publication, projection, adapter, journal)

            with journal.locked():
                events = journal.read_task_events(action_id)
            self.assertEqual("WRITE_INTENT", events[-1]["event"])
            self.assertNotIn("POST_ATTEMPTED", [item["event"] for item in events])
            self.assertEqual(0, adapter.events.count("task-intent"))

            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
                resume_action_id=action_id,
            )
            self.assertEqual("published", result["status"])
            self.assertEqual(1, adapter.events.count("task-intent"))

    def test_attempt_marker_crash_before_adapter_stays_pending_on_empty_read(self):
        old_head, new_head = "1" * 40, "2" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7194, old_head=old_head, new_head=new_head,
            )
            action_id = "task-intent:" + publication["publication_id"]
            append_event = journal.append_task_event

            def crash_after_durable_attempt(action, event, identity, evidence=None):
                record = append_event(action, event, identity, evidence)
                if event == "POST_ATTEMPTED":
                    raise RuntimeError("simulated crash before adapter invocation")
                return record

            with patch.object(journal, "append_task_event", side_effect=crash_after_durable_attempt):
                with self.assertRaisesRegex(RuntimeError, "simulated crash"):
                    self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(0, adapter.events.count("task-intent"))

            with self.assertRaisesRegex(
                publication_module.PublicationError, "empty readback remains pending",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_lost_post_response_resolves_exact_comment_without_second_post(self):
        class LostCommentResponseAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.post_attempts = 0

            def publish_task_intent(inner_self, value):
                inner_self.post_attempts += 1
                super().publish_task_intent(value)
                raise RuntimeError("simulated lost Task-comment POST response")

        old_head, new_head = "3" * 40, "4" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7195, old_head=old_head, new_head=new_head,
                adapter_type=LostCommentResponseAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task intent response uncertain",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(1, adapter.post_attempts)

            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
                resume_action_id=action_id,
            )
            self.assertEqual("published", result["status"])
            self.assertEqual(1, adapter.post_attempts)
            with journal.locked():
                events = journal.read_task_events(action_id)
            self.assertIn("POST_ATTEMPTED", [item["event"] for item in events])
            self.assertEqual("RESOLVED", events[-1]["event"])

    def test_denied_post_is_attempted_and_empty_retry_never_reposts(self):
        class DeniedCommentAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.post_attempts = 0

            def publish_task_intent(inner_self, value):
                inner_self.post_attempts += 1
                inner_self.events.append("task-intent")
                raise PermissionError("simulated GitHub comment-write denial")

        old_head, new_head = "5" * 40, "6" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7196, old_head=old_head, new_head=new_head,
                adapter_type=DeniedCommentAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task intent response uncertain",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(1, adapter.post_attempts)

            with self.assertRaisesRegex(
                publication_module.PublicationError, "empty readback remains pending",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(1, adapter.post_attempts)

    def test_wrong_server_author_after_post_remains_pending(self):
        class WrongAuthorAdapter(FakeAdapter):
            def publish_task_intent(inner_self, value):
                super().publish_task_intent(value)
                inner_self.publication_author_logins[value["publication_id"]] = "different-user"

        old_head, new_head = "7" * 40, "8" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7197, old_head=old_head, new_head=new_head,
                adapter_type=WrongAuthorAdapter,
            )
            with self.assertRaisesRegex(
                publication_module.PublicationError, "author or canonical content differs",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(1, adapter.events.count("task-intent"))
            with journal.locked():
                events = journal.read_task_events("task-intent:" + publication["publication_id"])
            self.assertEqual("READ_CONFLICT", events[-1]["event"])
            self.assertNotIn("RESOLVED", [item["event"] for item in events])

    def test_duplicate_matching_comments_after_attempt_remain_pending(self):
        class DuplicateAfterPostAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.post_attempts = 0

            def publish_task_intent(inner_self, value):
                inner_self.post_attempts += 1
                super().publish_task_intent(value)
                inner_self.publications.append(copy.deepcopy(value))

        old_head, new_head = "c" * 40, "d" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7222, old_head=old_head, new_head=new_head,
                adapter_type=DuplicateAfterPostAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaisesRegex(
                publication_module.PublicationError, "lookup is ambiguous",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            with self.assertRaisesRegex(
                publication_module.PublicationError, "lookup is ambiguous",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(1, adapter.post_attempts)
            with journal.locked():
                events = journal.read_task_events(action_id)
            self.assertEqual("READ_CONFLICT", events[-1]["event"])
            self.assertNotIn("RESOLVED", [item["event"] for item in events])

    def test_missing_duplicate_or_false_server_metadata_never_resolves(self):
        class MetadataAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.metadata_failure = None

            def find_task_publications(inner_self, publication_id):
                result = super().find_task_publications(publication_id)
                if result["publications"]:
                    mode = inner_self.metadata_failure
                    if mode == "missing-author":
                        result["publication_authors"] = []
                    elif mode == "duplicate-author":
                        result["publication_authors"] *= 2
                    elif mode == "false-author":
                        result["publication_authors"][0]["author_login"] = "other-user"
                    elif mode == "missing-body":
                        result["publication_bodies"] = []
                    elif mode == "duplicate-body":
                        result["publication_bodies"] *= 2
                    elif mode == "false-body":
                        result["publication_bodies"][0]["body"] += "\nchanged"
                return result

        old_head, new_head = "e" * 40, "f" * 40
        modes = (
            "missing-author", "duplicate-author", "false-author",
            "missing-body", "duplicate-body", "false-body",
        )
        for index, mode in enumerate(modes):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temp:
                publication, projection, adapter, journal = self.existing_update_case(
                    temp, 7230 + index, old_head=old_head, new_head=new_head,
                    adapter_type=MetadataAdapter,
                )
                adapter.metadata_failure = mode
                with self.assertRaisesRegex(
                    publication_module.PublicationError,
                    "author or canonical content differs",
                ):
                    self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
                action_id = "task-intent:" + publication["publication_id"]
                with journal.locked():
                    events = journal.read_task_events(action_id)
                self.assertEqual("READ_CONFLICT", events[-1]["event"])
                self.assertNotIn("RESOLVED", [item["event"] for item in events])
                self.assertEqual(1, adapter.events.count("task-intent"))

    def test_wrong_task_or_payload_action_cannot_resume_attempted_publication(self):
        old_head, new_head = "0" * 40, "1" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7238, old_head=old_head, new_head=new_head,
            )
            action_id = "task-intent:" + publication["publication_id"]
            def deny_post(_value):
                adapter.events.append("task-intent")
                raise PermissionError("simulated denied POST after attempt marker")
            adapter.publish_task_intent = deny_post
            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(1, adapter.events.count("task-intent"))

            changed_task, changed_projection = make_publication(
                7239, head="2" * 40, branch=publication["source_ref"],
                task_uid="task_" + "b" * 32,
            )
            changed_pr = {
                "repository": changed_task["repository"],
                "source_ref": changed_task["source_ref"],
                "target_ref": changed_task["target_ref"], "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1\n", "state": "open",
                "merged": False, "draft": True, "number": 7239,
            }
            changed_adapter = FakeAdapter(
                changed_task, changed_projection, initial_head=old_head,
                initial_pr=changed_pr,
            )
            changed_adapter.pr_binding = None
            with self.assertRaisesRegex(
                publication_module.PublicationError,
                "recovery selector does not match the exact publication action",
            ):
                self.run_publish_entrypoint(
                    temp, changed_task, changed_projection, changed_adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(0, changed_adapter.events.count("task-intent"))
            with journal.locked():
                old_events = journal.read_task_events(action_id)
            self.assertIn("POST_ATTEMPTED", [item["event"] for item in old_events])
            self.assertNotIn("RESOLVED", [item["event"] for item in old_events])

    def test_publisher_identity_drift_blocks_exact_action_recovery(self):
        old_head, new_head = "9" * 40, "a" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7198, old_head=old_head, new_head=new_head,
                adapter_type=FirstPublicationReadTimeoutAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            adapter.publisher_login = "changed-publisher"
            with self.assertRaisesRegex(
                publication_module.PublicationError, "action identity changed during recovery",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(1, adapter.publication_reads)
            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_task_pr_binding_drift_blocks_exact_action_through_publisher_ingress(self):
        old_head, new_head = "f" * 40, "0" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7202, old_head=old_head, new_head=new_head,
                adapter_type=FirstPublicationReadTimeoutAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            adapter.pr_binding = {"task_uid": UID, "pr_number": 9999}
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task is bound to another PR",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_publication_cli_accepts_only_exact_action_selector_not_force_or_proof(self):
        required = [
            "--worktree", "/tmp/worktree", "--repo", "eng-cc/oasis7",
            "--issue-number", "1", "--task-uid", UID, "--remote", "origin",
            "--source-ref", "feature/test", "--target-ref", "main",
            "--source-head", "a" * 40, "--target-oid", "b" * 40,
            "--projection", "/tmp/projection.json", "--body-file", "/tmp/body.md",
            "--task-helper", "/tmp/task-helper.py",
        ]
        action_id = "task-intent:sha256:" + "c" * 64
        with (
            patch.object(publish_module, "publish", return_value={"pr_url": "https://example/pr/1"}) as publish,
            patch.object(sys, "argv", ["pr-projection-publish", *required,
                                       "--resume-action-id", action_id]),
            redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(0, publish_module.main())
        self.assertEqual(action_id, publish.call_args.args[0].resume_action_id)

        for forbidden in ("--force", "--reset", "--post-was-not-sent"):
            with (
                patch.object(sys, "argv", ["pr-projection-publish", *required, forbidden]),
                redirect_stderr(io.StringIO()),
            ):
                with self.assertRaises(SystemExit):
                    publish_module.main()

    def test_unresolved_publisher_identity_fails_before_task_comment_lookup(self):
        class UnresolvedLoginAdapter(FakeAdapter):
            def resolve_publisher_login(self):
                raise RuntimeError("simulated gh api user failure")

        old_head, new_head = "b" * 40, "c" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7199, old_head=old_head, new_head=new_head,
                adapter_type=UnresolvedLoginAdapter,
            )
            with self.assertRaisesRegex(
                publication_module.PublicationError, "authenticated GitHub login read failed",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertNotIn("task-intent", adapter.events)
            with journal.locked():
                self.assertEqual([], journal.read_task_events())

    def test_legacy_or_truncated_task_post_history_never_authorizes_post(self):
        old_head, new_head = "d" * 40, "e" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7200, old_head=old_head, new_head=new_head,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with journal.locked():
                journal.intent(action_id, "publish_task_intent", {
                    "publication_id": publication["publication_id"], "task_uid": UID,
                })
            with self.assertRaisesRegex(
                publication_module.PublicationError, "legacy Task publication history is UNKNOWN",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(0, adapter.events.count("task-intent"))

        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7201, old_head=old_head, new_head=new_head,
                adapter_type=FirstPublicationReadTimeoutAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            with journal.task_events_path.open("ab") as stream:
                stream.write(b'{"partial":')
            with self.assertRaisesRegex(
                publication_module.PublicationError, "event journal is truncated",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_missing_task_post_event_sidecar_after_attempt_stays_pending(self):
        class DeniedCommentAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.post_attempts = 0

            def publish_task_intent(inner_self, value):
                inner_self.post_attempts += 1
                inner_self.events.append("task-intent")
                raise PermissionError("simulated uncertain GitHub comment-write result")

        old_head, new_head = "3" * 40, "4" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7242, old_head=old_head, new_head=new_head,
                adapter_type=DeniedCommentAdapter,
            )
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task intent response uncertain",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(1, adapter.post_attempts)
            self.assertTrue(journal.task_events_path.exists())

            # Model loss of the replaceable sidecar while keeping the exact
            # same publication identity and the remote lookup empty.
            journal.task_events_path.unlink()
            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)

            self.assertEqual(
                1, adapter.post_attempts,
                "a missing event sidecar must not turn an attempted action into a fresh POST",
            )

    def test_valid_prefix_rollback_before_attempt_marker_stays_pending(self):
        class DeniedCommentAdapter(FakeAdapter):
            def __init__(inner_self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                inner_self.post_attempts = 0

            def publish_task_intent(inner_self, value):
                inner_self.post_attempts += 1
                inner_self.events.append("task-intent")
                raise PermissionError("simulated uncertain GitHub comment-write result")

        old_head, new_head = "5" * 40, "6" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7243, old_head=old_head, new_head=new_head,
                adapter_type=DeniedCommentAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaisesRegex(
                publication_module.PublicationError, "Task intent response uncertain",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(1, adapter.post_attempts)

            with journal.locked():
                lines = journal.task_events_path.read_bytes().splitlines(keepends=True)
                events = [json.loads(line) for line in lines]
                action_events = [item["event"] for item in events
                                 if item["action_id"] == action_id]
                self.assertEqual(
                    ["READ_EMPTY", "WRITE_INTENT", "POST_ATTEMPTED",
                     "POST_RESPONSE_UNCERTAIN"], action_events,
                )
                # This is a complete, locally valid prefix: all remaining
                # sequence numbers and newline boundaries still verify.
                journal.task_events_path.write_bytes(b"".join(lines[:2]))

            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )

            self.assertEqual(
                1, adapter.post_attempts,
                "a valid prefix rollback must not authorize another POST for the same action",
            )

    def test_old_unanchored_root_and_missing_sidecar_empty_read_stays_unknown(self):
        old_head, new_head = "7" * 40, "8" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7244, old_head=old_head, new_head=new_head,
            )
            # Emulate a pre-anchor e54 journal with no Task action record and
            # no event sidecar. Its empty state cannot prove that history was
            # never present and later lost.
            with journal.locked():
                state = journal._read_or_create()
                state.pop("task_post_tail", None)
                journal_module._atomic_json(journal.path, state)

            with self.assertRaisesRegex(
                publication_module.PublicationError, "UNKNOWN|anchor|history",
            ):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_old_unanchored_exact_server_readback_resolves_without_post(self):
        old_head, new_head = "9" * 40, "a" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7245, old_head=old_head, new_head=new_head,
            )
            action_id = "task-intent:" + publication["publication_id"]
            adapter.publications.append(copy.deepcopy(publication))
            adapter.publication_author_logins[publication["publication_id"]] = adapter.publisher_login
            with journal.locked():
                journal.intent(action_id, "publish_task_intent", {
                    "publication_id": publication["publication_id"], "task_uid": UID,
                })
                state = journal._read_or_create()
                state.pop("task_post_tail", None)
                journal_module._atomic_json(journal.path, state)

            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
                resume_action_id=action_id,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(0, adapter.events.count("task-intent"))
            with journal.locked():
                events = journal.read_task_events(action_id)
                self.assertEqual("RESOLVED", events[-1]["event"])

    def test_unanchored_resolved_event_requires_exact_live_readback(self):
        old_head, new_head = "b" * 40, "c" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7248, old_head=old_head, new_head=new_head,
            )
            action_id = "task-intent:" + publication["publication_id"]
            pr_binding = {"state": "unbound"}
            with journal.locked():
                _login, identity = publication_module._task_intent_identity(
                    adapter, journal, publication, pr_binding,
                )
                journal.append_task_event(action_id, "READ_EMPTY", identity)
                journal.append_task_event(
                    action_id, "WRITE_INTENT", identity,
                    {"payload_sha256": identity["payload_sha256"]},
                )
                journal.append_task_event(
                    action_id, "POST_ATTEMPTED", identity,
                    {"payload_sha256": identity["payload_sha256"]},
                )
                journal.append_task_event(
                    action_id, "READ_MATCH", identity,
                    {"publication_id": publication["publication_id"],
                     "author_login": identity["publisher_login"],
                     "payload_sha256": identity["payload_sha256"]},
                )
                journal.append_task_event(
                    action_id, "RESOLVED", identity,
                    {"publication_id": publication["publication_id"]},
                )
                state = journal.read_action_state()
                state.pop("task_post_tail", None)
                journal_module._atomic_json(journal.path, state)

                with self.assertRaisesRegex(
                    publication_module.PublicationError, "UNKNOWN|pending|anchor",
                ):
                    publication_module._intent(
                        adapter, journal, publication, pr_binding=pr_binding,
                        resume_action_id=action_id,
                    )

            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_root_task_post_tail_survives_other_main_journal_updates(self):
        publication, _projection = make_publication(7246)
        with tempfile.TemporaryDirectory() as temp:
            journal = self.journal(temp, publication)
            action_id = "task-intent:" + publication["publication_id"]
            identity = {"publication_id": publication["publication_id"], "test": "tail-preservation"}

            with journal.locked():
                initial = journal.read()["task_post_tail"]
                self.assertEqual(0, initial["sequence"])
                journal.append_task_event(
                    action_id, "READ_FAILED", identity, {"phase": "prewrite"},
                )
                anchored = journal.read()["task_post_tail"]
                self.assertEqual(1, anchored["sequence"])

                other_action = "other-action:" + publication["publication_id"]
                journal.intent(other_action, "test_state_update", {"value": 1})
                journal.observe(other_action, {"value": 1}, phase="TEST_CONFIRMED")

                self.assertEqual(anchored, journal.read()["task_post_tail"])
                self.assertEqual(["READ_FAILED"], [
                    item["event"] for item in journal.read_task_events(action_id)
                ])

    def test_sidecar_fsync_before_root_anchor_crash_fails_closed(self):
        publication, _projection = make_publication(7247)
        with tempfile.TemporaryDirectory() as temp:
            journal = self.journal(temp, publication)
            action_id = "task-intent:" + publication["publication_id"]
            identity = {"publication_id": publication["publication_id"], "test": "crash-gap"}
            real_atomic_json = journal_module._atomic_json

            with journal.locked():
                self.assertEqual(0, journal.read()["task_post_tail"]["sequence"])

                def crash_before_anchor_replace(path, value):
                    if Path(path) == journal.path and "task_post_tail" in value:
                        raise OSError("injected crash before root tail anchor commit")
                    return real_atomic_json(path, value)

                with patch.object(journal_module, "_atomic_json", side_effect=crash_before_anchor_replace):
                    with self.assertRaises(journal_module.JournalError):
                        journal.append_task_event(
                            action_id, "READ_FAILED", identity, {"phase": "prewrite"},
                        )

            with journal.locked():
                with self.assertRaisesRegex(journal_module.JournalError, "tail|anchor|history"):
                    journal.read_task_events()

    def test_duplicate_task_post_event_history_fails_closed(self):
        old_head, new_head = "a" * 40, "b" * 40
        with tempfile.TemporaryDirectory() as temp:
            publication, projection, adapter, journal = self.existing_update_case(
                temp, 7240, old_head=old_head, new_head=new_head,
                adapter_type=FirstPublicationReadTimeoutAdapter,
            )
            action_id = "task-intent:" + publication["publication_id"]
            with self.assertRaises(publication_module.PublicationError):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            original = journal.task_events_path.read_bytes()
            first_event = original.splitlines(keepends=True)[0]
            with journal.task_events_path.open("ab") as stream:
                stream.write(first_event)
            with self.assertRaisesRegex(
                publication_module.PublicationError,
                "event identity or sequence is invalid",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    resume_action_id=action_id,
                )
            self.assertEqual(0, adapter.events.count("task-intent"))

    def test_publish_task_intent_rejects_altered_full_body_readback(self):
        publication, _projection = make_publication(7007)
        expected_body = publication_module.publication_comment(publication)
        issue = {"number": 123, "state": "open", "body": f"task_uid: {UID}\n"}

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            adapter.authenticated_login = "publisher"

            def altered_readback(*command, timeout=5.0, input_json=None):
                if command[:2] == ("api", f"repos/{publication['repository']}/issues/123"):
                    return json.dumps(issue)
                if command[:2] == ("issue", "comment"):
                    body_path = Path(command[command.index("--body-file") + 1])
                    self.assertEqual(expected_body, body_path.read_text(encoding="utf-8"))
                    return f"https://github.com/{publication['repository']}/issues/123#issuecomment-7007"
                if command[:2] == (
                    "api", f"repos/{publication['repository']}/issues/comments/7007",
                ):
                    return json.dumps({"id": 7007, "body": expected_body + "\nchanged",
                                       "user": {"login": "publisher"}})
                raise AssertionError(f"unexpected mocked GitHub call: {command!r}")

            with patch.object(adapter, "gh", side_effect=altered_readback):
                with self.assertRaisesRegex(RuntimeError, "Issue comment exact author/content readback failed"):
                    adapter.publish_task_intent(publication)

    def test_find_task_publication_on_later_paginated_comment_page(self):
        publication, _projection = make_publication(7008)
        issue = {"number": 123, "state": "open", "body": f"task_uid: {UID}\n"}
        first_page = [{"id": 10, "body": "older task evidence"}]
        second_page = [{
            "id": 11,
            "body": publication_module.publication_comment(publication),
            "created_at": "2026-09-30T12:00:00Z",
            "updated_at": "2026-09-30T12:00:00Z",
            "user": {"login": "publisher"},
        }]

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)

            def paginated_readback(*command, timeout=5.0, input_json=None):
                if command[:2] == ("api", f"repos/{publication['repository']}/issues/123"):
                    return json.dumps(issue)
                if command[:2] == (
                    "api", f"repos/{publication['repository']}/issues/123/comments?per_page=100",
                ):
                    self.assertIn("--paginate", command)
                    self.assertIn("--slurp", command)
                    return json.dumps([first_page, second_page])
                raise AssertionError(f"unexpected mocked GitHub call: {command!r}")

            with patch.object(adapter, "gh", side_effect=paginated_readback):
                result = adapter.find_task_publications(publication["publication_id"])

        self.assertIs(result["complete"], True)
        self.assertEqual([publication], result["publications"])
        self.assertEqual([{"publication_id": publication["publication_id"],
                           "author_login": "publisher"}], result["publication_authors"])
        self.assertEqual([{"publication_id": publication["publication_id"],
                           "body": publication_module.publication_comment(publication)}],
                         result["publication_bodies"])

    def test_pr_discovery_filters_large_history_server_side(self):
        repository = "eng-cc/oasis7"
        source_ref = "task/engineering-ci-parallel-reuse-c1"
        target_ref = "main"
        publication, _projection = make_publication(
            7002, repository=repository, branch=source_ref,
        )
        matching = {
            "number": 41, "html_url": f"https://github.com/{repository}/pull/41",
            "body": "Task: " + UID, "state": "open", "draft": True,
            "merged_at": None,
            "head": {"ref": source_ref, "sha": publication["source_head_oid"],
                     "repo": {"full_name": repository}},
            "base": {"ref": target_ref, "repo": {"full_name": repository}},
        }
        fork_match = copy.deepcopy(matching)
        fork_match["number"] = 42
        fork_match["head"]["repo"]["full_name"] = "fork/oasis7"
        large_history = [
            {"head": {"ref": f"old/branch-{index}"},
             "base": {"ref": "main"}}
            for index in range(10_000)
        ] + [matching, fork_match]
        expected_query = urlencode({
            "state": "all", "head": f"eng-cc:{source_ref}",
            "base": target_ref, "per_page": 100,
        })
        expected_path = f"repos/{repository}/pulls?{expected_query}"
        with tempfile.TemporaryDirectory() as temp:
            args = type("Args", (), {
                "repo": repository, "issue_number": 123,
                "task_uid": UID, "task_helper": str(Path(temp) / "github-project-task.py"),
            })()
            adapter = publish_module.GitHubPublicationAdapter(Path(temp), args, publication)

            def fake_gh(*command, timeout):
                endpoint = command[1] if len(command) > 1 else ""
                if endpoint != expected_path:
                    raise TimeoutError("scanning the fake repository PR history exceeds timeout")
                self.assertEqual(("api", expected_path, "--paginate", "--slurp"), command)
                self.assertEqual(4.5, timeout)
                filtered = [
                    item for item in large_history
                    if item.get("head", {}).get("ref") == source_ref
                    and item.get("base", {}).get("ref") == target_ref
                ]
                return json.dumps([filtered])

            with patch.object(adapter, "gh", side_effect=fake_gh) as gh:
                result = adapter.find_task_prs(UID, source_ref, target_ref, timeout_seconds=4.5)

        gh.assert_called_once()
        self.assertEqual([41], [item["number"] for item in result["pull_requests"]])

    def test_remote_source_ref_read_allows_slow_success_under_separate_budget(self):
        publication, _projection = make_publication(7003)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
                "remote": "origin",
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            observed_timeouts = []

            def simulated_remote_run(command, **kwargs):
                timeout = kwargs["timeout"]
                observed_timeouts.append(timeout)
                if 11.0 > timeout:
                    raise subprocess.TimeoutExpired(command, timeout)
                return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

            with patch.object(publish_module.subprocess, "run", side_effect=simulated_remote_run):
                self.assertIsNone(adapter.read_source_ref(publication["source_ref"]))

        self.assertEqual([30.0], observed_timeouts)
        self.assertGreater(publish_module.REMOTE_GIT_READ_TIMEOUT_SECONDS, 11.0)
        self.assertEqual(15.0, publish_module.LOCAL_COMMAND_TIMEOUT_SECONDS)

    def test_remote_source_ref_timeout_fails_closed(self):
        publication, _projection = make_publication(7004)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = type("Args", (), {
                "repo": publication["repository"], "issue_number": 123,
                "task_uid": UID, "task_helper": str(root / "github-project-task.py"),
                "remote": "origin",
            })()
            adapter = publish_module.GitHubPublicationAdapter(root, args, publication)
            observed_timeouts = []

            def simulated_timeout(command, **kwargs):
                observed_timeouts.append(kwargs["timeout"])
                raise subprocess.TimeoutExpired(command, kwargs["timeout"])

            with patch.object(publish_module.subprocess, "run", side_effect=simulated_timeout):
                with self.assertRaisesRegex(publish_module.PublishInputError, "command failed: git"):
                    adapter.read_source_ref(publication["source_ref"])

        self.assertEqual([30.0], observed_timeouts)

    def test_30_ordered_create_publications(self):
        with tempfile.TemporaryDirectory() as temp:
            for index in range(30):
                publication, projection = make_publication(index)
                adapter = FakeAdapter(publication, projection, initial_head=None)
                result = publication_module.publish_create(
                    adapter, self.journal(temp, publication), publication=publication,
                    projection=projection, body=f"Refs #1\n\n",
                )
                self.assertEqual("published", result["status"])
                self.assertEqual(1, result["pr_number"])
                self.assertLess(adapter.events.index("task-intent"), adapter.events.index("push"))
                self.assertLess(adapter.events.index("push"), adapter.events.index("create-pr"))
                self.assertLess(adapter.events.index("create-pr"), adapter.events.index("record-pr"))
                self.assertLess(adapter.events.index("record-pr"), adapter.events.index("publish-reciprocal"))

    def test_create_rejects_task_bound_pr_missing_from_exact_discovery_before_writes(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(490)
            other_pr = {
                "repository": publication["repository"], "source_ref": "task/older-branch",
                "target_ref": publication["target_ref"], "head_oid": publication["source_head_oid"],
                "body": "old body", "state": "open", "merged": False,
                "draft": True, "number": 99,
            }
            adapter = FakeAdapter(publication, projection, initial_pr=other_pr)
            adapter.pr_binding = {"task_uid": UID, "pr_number": 99}
            with self.assertRaisesRegex(
                publication_module.PublicationError, "TASK_IDENTITY_CONFLICT",
            ):
                publication_module.publish_create(
                    adapter, self.journal(temp, publication), publication=publication,
                    projection=projection, body=f"Task: {UID}\nRefs #1",
                )
            self.assertNotIn("task-intent", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_create_reconciles_exact_existing_task_bound_pr_without_duplicate(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(491)
            _, marker = publication_module.prepare(
                task_uid=UID, source_head_oid=publication["source_head_oid"],
                scope_base_oid=SCOPE, projection_digest=publication["projection_digest"],
            )
            body = publication_module.replace_projection_marker(
                f"Task: {UID}\nRefs #1", marker,
            )
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": publication["source_head_oid"],
                "body": body, "state": "open", "merged": False,
                "draft": True, "number": 17,
            }
            adapter = FakeAdapter(publication, projection,
                                  initial_head=publication["source_head_oid"], initial_pr=pr)
            result = publication_module.publish_create(
                adapter, self.journal(temp, publication), publication=publication,
                projection=projection, body=f"Task: {UID}\nRefs #1",
            )
            self.assertEqual(17, result["pr_number"])
            self.assertEqual(1, len(adapter.prs))
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertEqual(1, adapter.events.count("publish-reciprocal"))

    def test_30_ordered_existing_pr_updates(self):
        with tempfile.TemporaryDirectory() as temp:
            for index in range(30):
                old_head = f"{index + 100:040x}"
                new_head = f"{index + 200:040x}"
                publication, projection = make_publication(index + 1000, head=new_head,
                                                           branch=f"feature/update-{index}")
                old_contract, old_marker = publication_module.prepare(
                    task_uid=UID, source_head_oid=old_head, scope_base_oid=SCOPE,
                    projection_digest=digest({"old": index}),
                )
                pr = {"repository": publication["repository"], "source_ref": publication["source_ref"],
                      "target_ref": publication["target_ref"], "head_oid": old_head,
                      "body": "Refs #1\n\n" + old_marker, "state": "open", "merged": False,
                      "draft": True, "number": index + 1}
                adapter = FakeAdapter(publication, projection, initial_head=old_head, initial_pr=pr)
                result = publication_module.publish_update(
                    adapter, self.journal(temp, publication), publication=publication,
                    projection=projection, pr_number=index + 1, old_head_oid=old_head,
                    body=pr["body"],
                )
                self.assertEqual("published", result["status"])
                self.assertLess(adapter.events.index("patch-pr"), adapter.events.index("push"))
                self.assertEqual(new_head, adapter.prs[0]["head_oid"])
                self.assertEqual(result["binding"]["binding_digest"], adapter.bindings[0]["binding_digest"])

    def test_existing_ready_update_keeps_exact_bound_nondraft_pr(self):
        """A ready update is the same-PR H1 path with a pinned false draft bit."""
        with tempfile.TemporaryDirectory() as temp:
            old_head = "c" * 40
            new_head = "d" * 40
            publication, projection = make_publication(1090, head=new_head)
            _, old_marker = publication_module.prepare(
                task_uid=UID, source_head_oid=old_head, scope_base_oid=SCOPE,
                projection_digest=digest({"ready-update": "old"}),
            )
            pr = {
                "repository": publication["repository"],
                "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"],
                "head_oid": old_head,
                "body": f"Manually retained summary\n\nTask: {UID}\nRefs #1\n\n{old_marker}",
                "state": "open", "merged": False, "draft": False, "number": 109,
            }
            adapter = FakeAdapter(
                publication, projection, initial_head=old_head, initial_pr=pr,
            )
            adapter.ready_update_admitted = True
            journal = self.journal(temp, publication)

            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
                existing_ready_update=True,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(109, result["pr_number"])
            self.assertEqual(1, len(adapter.prs), "ready update must reuse the exact existing PR")
            self.assertFalse(adapter.prs[0]["draft"], "the ready PR must remain non-draft")
            self.assertEqual(new_head, adapter.prs[0]["head_oid"])
            self.assertTrue(adapter.prs[0]["body"].startswith("Manually retained summary\n\n"))
            self.assertNotIn("create-pr", adapter.events)
            self.assertLess(adapter.events.index("task-intent"), adapter.events.index("patch-pr"))
            self.assertLess(adapter.events.index("patch-pr"), adapter.events.index("push"))
            self.assertLess(adapter.events.index("push"), adapter.events.index("record-pr"))
            self.assertLess(adapter.events.index("record-pr"), adapter.events.index("publish-reciprocal"))
            with journal.locked():
                actions = journal.read()["actions"]
            pinned_state = next(item for item in actions if item["action_id"] == "update-state:" + publication["publication_id"])
            push = next(item for item in actions if item["action_id"] == "push:" + publication["publication_id"])
            self.assertEqual({
                "pr_number": 109,
                "expected_draft": False,
                "existing_ready_update": True,
                "old_head_oid": old_head,
            }, pinned_state["expected"])
            self.assertEqual(old_head, push["expected"]["lease_oid"])

    def test_ready_update_preserves_original_lease_after_record_response_loss(self):
        class LostReadyRecordAdapter(FakeAdapter):
            def __init__(self, publication, projection, *, initial_head, initial_pr):
                super().__init__(publication, projection, initial_head=initial_head, initial_pr=initial_pr)
                self.ready_update_admitted = True
                self.record_attempts = 0

            def record_pr(self, task_uid, number, publication_id):
                self.record_attempts += 1
                if self.record_attempts == 1:
                    self.events.append("record-pr")
                    raise RuntimeError("simulated lost ready-update Task writer response")
                super().record_pr(task_uid, number, publication_id)

        with tempfile.TemporaryDirectory() as temp:
            old_head = "5" * 40
            new_head = "6" * 40
            publication, projection = make_publication(1094, head=new_head)
            _, old_marker = publication_module.prepare(
                task_uid=UID, source_head_oid=old_head, scope_base_oid=SCOPE,
                projection_digest=digest({"ready-retry": "old"}),
            )
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1\n\n{old_marker}",
                "state": "open", "merged": False, "draft": False, "number": 112,
            }
            adapter = LostReadyRecordAdapter(
                publication, projection, initial_head=old_head, initial_pr=pr,
            )
            journal = self.journal(temp, publication)

            with self.assertRaisesRegex(publication_module.PublicationError, "record-pr transition did not confirm"):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    existing_ready_update=True,
                )
            self.assertEqual(new_head, adapter.prs[0]["head_oid"])
            with journal.locked():
                first_actions = journal.read()["actions"]
            pinned = next(item for item in first_actions if item["action_id"] == "update-state:" + publication["publication_id"])
            push = next(item for item in first_actions if item["action_id"] == "push:" + publication["publication_id"])
            self.assertEqual(old_head, pinned["expected"]["old_head_oid"])
            self.assertEqual(old_head, push["expected"]["lease_oid"])

            adapter.events.clear()
            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
                existing_ready_update=True,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(2, adapter.record_attempts)
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(new_head, adapter.prs[0]["head_oid"])
            self.assertNotIn("push", adapter.events, "H1 recovery must reuse the original lease without a second push")
            self.assertNotIn("create-pr", adapter.events)
            self.assertEqual(1, len(adapter.bindings))

    def test_ready_update_h1_without_pinned_admission_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            new_head = "7" * 40
            publication, projection = make_publication(1095, head=new_head)
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": new_head,
                "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
                "draft": False, "number": 113,
            }
            adapter = FakeAdapter(publication, projection, initial_head=new_head, initial_pr=pr)
            adapter.ready_update_admitted = True
            journal = self.journal(temp, publication)

            with self.assertRaisesRegex(publication_module.PublicationError, "H1 recovery lacks pinned update identity"):
                publication_module.publish_update(
                    adapter, journal, publication=publication, projection=projection,
                    pr_number=113, old_head_oid=new_head, body=pr["body"],
                    expected_draft=False, existing_ready_update=True,
                )

            self.assertNotIn("task-intent", adapter.events)
            self.assertNotIn("patch-pr", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_ready_update_rechecks_false_draft_after_metadata_patch(self):
        class DraftFlipAdapter(FakeAdapter):
            def patch_pr_body(self, repository, number, body):
                super().patch_pr_body(repository, number, body)
                self.prs[0]["draft"] = True

        with tempfile.TemporaryDirectory() as temp:
            old_head = "8" * 40
            publication, projection = make_publication(1096, head="9" * 40)
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
                "draft": False, "number": 114,
            }
            adapter = DraftFlipAdapter(publication, projection, initial_head=old_head, initial_pr=pr)
            adapter.ready_update_admitted = True
            journal = self.journal(temp, publication)

            with self.assertRaisesRegex(
                publication_module.PublicationError, "live PR repository/ref/head/state identity mismatch",
            ):
                publication_module.publish_update(
                    adapter, journal, publication=publication, projection=projection,
                    pr_number=114, old_head_oid=old_head, body=pr["body"],
                    expected_draft=False, existing_ready_update=True,
                )

            self.assertIn("patch-pr", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_ready_update_rechecks_nondraft_pr_after_task_record(self):
        class DraftFlipAfterRecordAdapter(FakeAdapter):
            def __init__(self, publication, projection, *, initial_head, initial_pr):
                super().__init__(publication, projection, initial_head=initial_head, initial_pr=initial_pr)
                self.ready_update_admitted = True

            def record_pr(self, task_uid, number, publication_id):
                super().record_pr(task_uid, number, publication_id)
                self.prs[0]["draft"] = True

        with tempfile.TemporaryDirectory() as temp:
            old_head = "a" * 40
            publication, projection = make_publication(1097, head="b" * 40)
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1\n", "state": "open", "merged": False,
                "draft": False, "number": 116,
            }
            adapter = DraftFlipAfterRecordAdapter(
                publication, projection, initial_head=old_head, initial_pr=pr,
            )
            journal = self.journal(temp, publication)

            with self.assertRaisesRegex(
                publication_module.PublicationError, "live PR repository/ref/head/state identity mismatch",
            ):
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    existing_ready_update=True,
                )

            self.assertIn("record-pr", adapter.events)
            self.assertIn("read-pr", adapter.events, "record completion must be followed by PR state readback")
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_existing_ready_update_without_existing_pr_fails_before_create(self):
        with tempfile.TemporaryDirectory() as temp:
            old_head = "e" * 40
            publication, projection = make_publication(1091, head="f" * 40)
            adapter = FakeAdapter(publication, projection, initial_head=old_head)
            journal = self.journal(temp, publication)

            rejected = False
            try:
                self.run_publish_entrypoint(
                    temp, publication, projection, adapter, journal,
                    existing_ready_update=True,
                )
            except publish_module.PublishInputError as exc:
                rejected = True
                self.assertRegex(str(exc), "existing-ready-update.*existing PR")

            self.assertNotIn("task-intent", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)
            self.assertTrue(rejected, "ready-update mode must reject when exact existing PR discovery is empty")

    def test_existing_ready_update_rejects_missing_live_admission_marker(self):
        with tempfile.TemporaryDirectory() as temp:
            old_head = "1" * 40
            new_head = "2" * 40
            publication, projection = make_publication(1092, head=new_head)
            pr = {
                "repository": publication["repository"],
                "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"],
                "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1\n",
                "state": "open", "merged": False, "draft": False, "number": 110,
            }
            adapter = FakeAdapter(publication, projection, initial_head=old_head, initial_pr=pr)
            journal = self.journal(temp, publication)

            with self.assertRaisesRegex(
                publication_module.PublicationError, "live existing-ready-update admission is missing",
            ):
                publication_module.publish_update(
                    adapter, journal, publication=publication, projection=projection,
                    pr_number=110, old_head_oid=old_head, body=pr["body"],
                    expected_draft=False, existing_ready_update=True,
                )

            self.assertNotIn("task-intent", adapter.events)
            self.assertNotIn("patch-pr", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_expected_nondraft_without_explicit_ready_mode_is_rejected_before_reads(self):
        with tempfile.TemporaryDirectory() as temp:
            old_head = "3" * 40
            publication, projection = make_publication(1093, head="4" * 40)
            pr = {
                "repository": publication["repository"],
                "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"],
                "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1\n",
                "state": "open", "merged": False, "draft": False, "number": 111,
            }
            adapter = FakeAdapter(publication, projection, initial_head=old_head, initial_pr=pr)
            journal = self.journal(temp, publication)

            with self.assertRaisesRegex(
                publication_module.PublicationError, "explicit ready update and draft expectation disagree",
            ):
                publication_module.publish_update(
                    adapter, journal, publication=publication, projection=projection,
                    pr_number=111, old_head_oid=old_head, body=pr["body"],
                    expected_draft=False,
                )

            self.assertEqual([], adapter.events, "unadmitted expected_draft=False must not read or write live state")

    def test_lost_create_response_recovers_by_exact_readback_without_second_post(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(500)
            adapter = FakeAdapter(publication, projection)
            original = adapter.create_draft_pr
            def lose_response(*args):
                original(*args)
                raise OSError("lost response")
            adapter.create_draft_pr = lose_response
            result = publication_module.publish_create(
                adapter, self.journal(temp, publication), publication=publication,
                projection=projection, body="Refs #1",
            )
            self.assertEqual("published", result["status"])
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(1, adapter.events.count("create-pr"))

    def test_entrypoint_no_pr_retry_recovers_push_after_lost_readback(self):
        for lease_oid in (None, "8" * 40):
            with self.subTest(lease_oid=lease_oid), tempfile.TemporaryDirectory() as temp:
                publication, projection = make_publication(507)
                adapter = FakeAdapter(publication, projection, initial_head=lease_oid)
                journal = self.journal(temp, publication)
                original_read = adapter.read_source_ref

                def lose_pushed_readback(ref):
                    if "push" in adapter.events:
                        raise OSError("lost push readback")
                    return original_read(ref)

                adapter.read_source_ref = lose_pushed_readback
                with self.assertRaisesRegex(publication_module.PublicationError, "NETWORK_UNCERTAIN"):
                    self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
                self.assertEqual(publication["source_head_oid"], adapter.source_ref)
                self.assertEqual([], adapter.prs, "push readback failed before PR creation")
                with journal.locked():
                    push = next(item for item in journal.read()["actions"]
                                if item["action_id"] == "push:" + publication["publication_id"])
                self.assertEqual(lease_oid, push["expected"]["lease_oid"])
                self.assertEqual("uncertain", push["state"])
                adapter.read_source_ref = original_read
                adapter.events.clear()

                result = self.run_publish_entrypoint(temp, publication, projection, adapter, journal)

                self.assertEqual("published", result["status"])
                self.assertEqual(1, len(adapter.prs))
                self.assertEqual(1, adapter.events.count("create-pr"))
                self.assertNotIn("push", adapter.events)
                self.assertIn("read-source", adapter.events)
                with journal.locked():
                    recovered = next(item for item in journal.read()["actions"]
                                     if item["action_id"] == push["action_id"])
                self.assertEqual(push["expected"], recovered["expected"])
                self.assertEqual("observed", recovered["state"])

    def test_entrypoint_no_pr_retry_rejects_remote_conflict(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(508)
            adapter = FakeAdapter(publication, projection, initial_head="9" * 40)
            journal = self.journal(temp, publication)
            with journal.locked():
                journal.intent("push:" + publication["publication_id"], "push_source_ref", {
                    "source_ref": publication["source_ref"],
                    "new_oid": publication["source_head_oid"], "lease_oid": None,
                })
            with self.assertRaisesRegex(publication_module.PublicationError, "SOURCE_SUPERSEDED"):
                self.run_publish_entrypoint(temp, publication, projection, adapter, journal)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertNotIn("record-pr", adapter.events)
            self.assertNotIn("publish-reciprocal", adapter.events)

    def test_entrypoint_retries_exact_published_pr_without_reusing_push_lease(self):
        with tempfile.TemporaryDirectory() as temp:
            lease_oid = "8" * 40
            publication, projection = make_publication(506)
            adapter = FakeAdapter(publication, projection, initial_head=lease_oid)
            journal = self.journal(temp, publication)
            initial = publication_module.publish_create(
                adapter, journal, publication=publication, projection=projection,
                body=f"Task: {UID}\nRefs #1", expected_remote_oid=lease_oid,
            )
            self.assertEqual("published", initial["status"])
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(1, adapter.pr_binding["pr_number"])
            adapter.events.clear()

            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(1, result["pr_number"])
            self.assertEqual(1, len(adapter.prs))
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertEqual(1, adapter.events.count("record-pr"))
            self.assertEqual(1, len(adapter.bindings), "exact reciprocal comment is reused")

    def test_observed_record_pr_retry_rejects_drift_with_matching_issue_url(self):
        class LifecycleDriftAdapter(FakeAdapter):
            def __init__(self, publication, projection, *, initial_head):
                super().__init__(publication, projection, initial_head=initial_head)
                self.lifecycle_state = "committed/verification"

            def record_pr(self, task_uid, number, publication_id):
                if self.lifecycle_state != "committed/verification":
                    self.events.append("record-pr")
                    raise RuntimeError("canonical helper rejected live Task lifecycle drift")
                super().record_pr(task_uid, number, publication_id)

        with tempfile.TemporaryDirectory() as temp:
            lease_oid = "9" * 40
            publication, projection = make_publication(508)
            adapter = LifecycleDriftAdapter(
                publication, projection, initial_head=lease_oid,
            )
            journal = self.journal(temp, publication)
            initial = publication_module.publish_create(
                adapter, journal, publication=publication, projection=projection,
                body=f"Task: {UID}\nRefs #1", expected_remote_oid=lease_oid,
            )
            self.assertEqual("published", initial["status"])
            self.assertEqual({"task_uid": UID, "pr_number": 1}, adapter.pr_binding)
            self.assertEqual(1, len(adapter.bindings))

            # The Issue URL still resolves to the exact PR, but lifecycle truth
            # drifted after the journal had recorded successful record-pr.
            adapter.lifecycle_state = "execution"
            adapter.events.clear()
            with self.assertRaisesRegex(
                publication_module.PublicationError,
                "NETWORK_UNCERTAIN: record-pr transition did not confirm",
            ):
                publication_module.publish_create(
                    adapter, journal, publication=publication, projection=projection,
                    body=f"Task: {UID}\nRefs #1",
                )

            self.assertEqual({"task_uid": UID, "pr_number": 1}, adapter.pr_binding)
            self.assertEqual(1, adapter.events.count("record-pr"))
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(1, len(adapter.bindings), "retry does not duplicate reciprocal comment")

    def test_entrypoint_recovers_exact_created_pr_before_task_url_write(self):
        class FirstRecordPrFailureAdapter(FakeAdapter):
            def __init__(self, publication, projection, *, initial_head):
                super().__init__(publication, projection, initial_head=initial_head)
                self.record_attempts = 0

            def record_pr(self, task_uid, number, publication_id):
                self.record_attempts += 1
                if self.record_attempts == 1:
                    self.events.append("record-pr")
                    raise RuntimeError("simulated interruption before Task URL write")
                super().record_pr(task_uid, number, publication_id)

        with tempfile.TemporaryDirectory() as temp:
            lease_oid = "7" * 40
            publication, projection = make_publication(507)
            adapter = FirstRecordPrFailureAdapter(
                publication, projection, initial_head=lease_oid,
            )
            journal = self.journal(temp, publication)
            with self.assertRaisesRegex(publication_module.PublicationError, "NETWORK_UNCERTAIN"):
                publication_module.publish_create(
                    adapter, journal, publication=publication, projection=projection,
                    body=f"Task: {UID}\nRefs #1", expected_remote_oid=lease_oid,
                )
            self.assertEqual(1, len(adapter.prs))
            self.assertIsNone(adapter.pr_binding)
            adapter.events.clear()

            result = self.run_publish_entrypoint(
                temp, publication, projection, adapter, journal,
            )

            self.assertEqual("published", result["status"])
            self.assertEqual(1, result["pr_number"])
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(1, adapter.pr_binding["pr_number"])
            self.assertEqual(2, adapter.record_attempts)
            self.assertNotIn("push", adapter.events)
            self.assertNotIn("create-pr", adapter.events)

    def test_uncertain_record_pr_retries_full_transition_when_issue_binding_is_visible(self):
        class PostWriteReadbackFailureAdapter(FakeAdapter):
            def __init__(self, publication, projection):
                super().__init__(publication, projection)
                self.binding_reads = 0

            def read_task_pr_binding(self, task_uid):
                self.events.append("read-task-pr-binding")
                self.binding_reads += 1
                if self.binding_reads in (1, 2):
                    return {"task_uid": task_uid, "pr_number": None}
                if self.binding_reads == 3:
                    raise RuntimeError("simulated Task PR binding readback failure")
                return {"task_uid": task_uid, "pr_number": 1}

        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(504)
            adapter = PostWriteReadbackFailureAdapter(publication, projection)
            journal = self.journal(temp, publication)
            with self.assertRaisesRegex(
                publication_module.PublicationError, "NETWORK_UNCERTAIN",
            ):
                publication_module.publish_create(
                    adapter, journal, publication=publication,
                    projection=projection, body="Task: " + UID + "\nRefs #1",
                )

            result = publication_module.publish_create(
                adapter, journal, publication=publication,
                projection=projection, body="Task: " + UID + "\nRefs #1",
            )
            self.assertEqual("published", result["status"])
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(1, adapter.events.count("create-pr"))
            self.assertEqual(2, adapter.events.count("record-pr"))
            self.assertEqual(1, adapter.events.count("publish-reciprocal"))

    def test_partial_record_pr_failure_cannot_be_confirmed_by_issue_url_alone(self):
        class PartialRecordPrAdapter(FakeAdapter):
            def __init__(self, publication, projection):
                super().__init__(publication, projection)
                self.mapping_complete = False

            def record_pr(self, task_uid, number, publication_id):
                super().record_pr(task_uid, number, publication_id)
                if self.events.count("record-pr") == 1:
                    raise RuntimeError("simulated failure after Issue body write, before task mapping")
                self.mapping_complete = True

            def publish_reciprocal_binding(self, value):
                if not self.mapping_complete:
                    raise AssertionError("publication advanced before complete record-pr recovery")
                super().publish_reciprocal_binding(value)

        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(505)
            adapter = PartialRecordPrAdapter(publication, projection)
            journal = self.journal(temp, publication)
            with self.assertRaisesRegex(publication_module.PublicationError, "NETWORK_UNCERTAIN"):
                publication_module.publish_create(
                    adapter, journal, publication=publication,
                    projection=projection, body="Task: " + UID + "\nRefs #1",
                )
            self.assertEqual(1, adapter.prs[0]["number"])
            self.assertEqual(1, adapter.pr_binding["pr_number"])
            self.assertEqual([], adapter.bindings)
            self.assertFalse(adapter.mapping_complete)

            result = publication_module.publish_create(
                adapter, journal, publication=publication,
                projection=projection, body="Task: " + UID + "\nRefs #1",
            )
            self.assertEqual("published", result["status"])
            self.assertTrue(adapter.mapping_complete)
            self.assertEqual(2, adapter.events.count("record-pr"))
            self.assertEqual(1, adapter.events.count("publish-reciprocal"))

    def test_restart_after_create_effect_reuses_the_single_matching_pr(self):
        class SimulatedCrash(BaseException):
            pass
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(503)
            adapter = FakeAdapter(publication, projection)
            original = adapter.create_draft_pr
            def crash_after_create(*args):
                original(*args)
                raise SimulatedCrash()
            adapter.create_draft_pr = crash_after_create
            journal = self.journal(temp, publication)
            with self.assertRaises(SimulatedCrash):
                publication_module.publish_create(
                    adapter, journal, publication=publication,
                    projection=projection, body="Task: " + UID + "\nRefs #1",
                )
            adapter.create_draft_pr = original
            result = publication_module.publish_create(
                adapter, journal, publication=publication,
                projection=projection, body="Task: " + UID + "\nRefs #1",
            )
            self.assertEqual("published", result["status"])
            self.assertEqual(1, len(adapter.prs))
            self.assertEqual(1, adapter.events.count("create-pr"))

    def test_delayed_pr_number_uses_bounded_visibility_wait(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(502)
            adapter = FakeAdapter(publication, projection)
            original = adapter.find_task_prs
            reads = []
            def delayed(*args, **kwargs):
                reads.append(args)
                if len(reads) <= 2:
                    return {"complete": True, "pull_requests": []}
                return original(*args, **kwargs)
            adapter.find_task_prs = delayed
            clock = FakeClock()
            result = publication_module.publish_create(
                adapter, self.journal(temp, publication), publication=publication,
                projection=projection, body="Refs #1", clock=clock.clock, sleep=clock.sleep,
            )
            self.assertEqual("published", result["status"])
            self.assertEqual(1, result["pr_number"])
            self.assertEqual([2.0], clock.sleeps)

    def test_h2_conflict_stops_update_before_patch(self):
        with tempfile.TemporaryDirectory() as temp:
            publication, projection = make_publication(501, head="e" * 40)
            pr = {"repository": publication["repository"], "source_ref": publication["source_ref"],
                  "target_ref": publication["target_ref"], "head_oid": "f" * 40,
                  "body": "old", "state": "open", "merged": False, "draft": True, "number": 1}
            adapter = FakeAdapter(publication, projection, initial_head="f" * 40, initial_pr=pr)
            with self.assertRaisesRegex(publication_module.PublicationError, "SOURCE_SUPERSEDED"):
                publication_module.publish_update(
                    adapter, self.journal(temp, publication), publication=publication,
                    projection=projection, pr_number=1, old_head_oid="d" * 40, body="new",
                )
            self.assertNotIn("patch-pr", adapter.events)
            self.assertNotIn("push", adapter.events)

    def test_update_rejects_different_task_bound_pr_before_writes(self):
        with tempfile.TemporaryDirectory() as temp:
            old_head = "a" * 40
            publication, projection = make_publication(492, head="b" * 40)
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": old_head,
                "body": f"Task: {UID}\nRefs #1", "state": "open", "merged": False,
                "draft": True, "number": 1,
            }
            adapter = FakeAdapter(publication, projection, initial_head=old_head, initial_pr=pr)
            adapter.pr_binding = {"task_uid": UID, "pr_number": 99}
            with self.assertRaisesRegex(
                publication_module.PublicationError, "TASK_IDENTITY_CONFLICT",
            ):
                publication_module.publish_update(
                    adapter, self.journal(temp, publication), publication=publication,
                    projection=projection, pr_number=1, old_head_oid=old_head, body=pr["body"],
                )
            self.assertEqual(["read-task-pr-binding"], adapter.events)

    def test_update_preserves_live_manual_pr_body(self):
        with tempfile.TemporaryDirectory() as temp:
            old_head = "c" * 40
            new_head = "d" * 40
            publication, projection = make_publication(493, head=new_head)
            _, old_marker = publication_module.prepare(
                task_uid=UID, source_head_oid=old_head, scope_base_oid=SCOPE,
                projection_digest=digest({"old": "projection"}),
            )
            manual_prefix = f"Manually edited PR summary\n\nTask: {UID}\nRefs #1"
            pr = {
                "repository": publication["repository"], "source_ref": publication["source_ref"],
                "target_ref": publication["target_ref"], "head_oid": old_head,
                "body": manual_prefix + "\n\n" + old_marker,
                "state": "open", "merged": False, "draft": True, "number": 1,
            }
            adapter = FakeAdapter(publication, projection, initial_head=old_head, initial_pr=pr)
            adapter.pr_binding = {"task_uid": UID, "pr_number": 1}
            publication_module.publish_update(
                adapter, self.journal(temp, publication), publication=publication,
                projection=projection, pr_number=1, old_head_oid=old_head,
                body=f"Generated replacement summary\n\nTask: {UID}\nRefs #1",
            )
            self.assertTrue(adapter.prs[0]["body"].startswith(manual_prefix))
            self.assertNotIn("Generated replacement summary", adapter.prs[0]["body"])
            self.assertEqual(1, adapter.prs[0]["body"].count("oasis7-ci-impact-publication:v2"))

    def test_h1_with_stale_body_repairs_metadata_without_moving_head(self):
        with tempfile.TemporaryDirectory() as temp:
            head = "9" * 40
            publication, projection = make_publication(5020, head=head)
            _old, old_marker = publication_module.prepare(
                task_uid=UID, source_head_oid="8" * 40, scope_base_oid=SCOPE,
                projection_digest=digest({"old": "projection"}),
            )
            pr = {"repository": publication["repository"], "source_ref": publication["source_ref"],
                  "target_ref": publication["target_ref"], "head_oid": head,
                  "body": "Task: " + UID + "\nRefs #1\n\n" + old_marker,
                  "state": "open", "merged": False, "draft": True, "number": 1}
            adapter = FakeAdapter(publication, projection, initial_head=head, initial_pr=pr)
            result = publication_module.publish_update(
                adapter, self.journal(temp, publication), publication=publication,
                projection=projection, pr_number=1, old_head_oid=head,
                body=pr["body"],
            )
            self.assertEqual("published", result["status"])
            self.assertIn("patch-pr", adapter.events)
            self.assertNotIn("push", adapter.events)
            self.assertEqual(head, adapter.prs[0]["head_oid"])

    def test_100_stable_target_snapshots_resolve_reciprocal_binding(self):
        clock = FakeClock()
        for index in range(100):
            publication, projection = make_publication(index + 2000)
            pr_number = index + 1
            url = f"https://github.com/{publication['repository']}/pull/{pr_number}"
            binding = publication_module.build_publication_binding(publication, pr_number, url)
            _value, body = publication_module.prepare(
                task_uid=UID, source_head_oid=publication["source_head_oid"],
                scope_base_oid=SCOPE, projection_digest=publication["projection_digest"],
            )
            snapshot = {"complete": True, "publication": publication, "binding": binding,
                        "body": body, "repository": publication["repository"],
                        "repository_id": publication["repository_id"],
                        "source_repository_id": publication["source_repository_id"],
                        "source_ref": publication["source_ref"], "target_ref": publication["target_ref"],
                        "head_oid": publication["source_head_oid"], "state": "open",
                        "merged": False, "pr_number": pr_number}
            timeouts = []
            result = resolver_module.wait_for_binding(
                lambda timeout, snapshot=snapshot: (timeouts.append(timeout) or snapshot),
                task_uid=UID, source_head_oid=publication["source_head_oid"],
                scope_base_oid=SCOPE, repository=publication["repository"],
                pr_number=pr_number, planner_config_sha256=CONFIG,
                clock=clock.clock, sleep=clock.sleep,
            )
            self.assertEqual(projection["projection_digest"], result["projection_digest"])
            self.assertEqual([5.0, 5.0], timeouts)
        self.assertEqual([], clock.sleeps)

    def test_complete_snapshot_requires_publication_and_reciprocal_binding(self):
        publication, _projection = make_publication(2500)
        _, body = publication_module.prepare(
            task_uid=UID, source_head_oid=publication["source_head_oid"],
            scope_base_oid=SCOPE, projection_digest=publication["projection_digest"],
        )
        binding = publication_module.build_publication_binding(
            publication, 1, f"https://github.com/{publication['repository']}/pull/1",
        )
        complete = {
            "complete": True, "publication": publication, "binding": binding,
            "body": body, "repository": publication["repository"],
            "repository_id": publication["repository_id"],
            "source_repository_id": publication["source_repository_id"],
            "source_ref": publication["source_ref"], "target_ref": publication["target_ref"],
            "head_oid": publication["source_head_oid"], "state": "open",
            "merged": False, "pr_number": 1,
        }
        for missing in ("publication", "binding"):
            snapshot = dict(complete)
            snapshot.pop(missing)
            with self.subTest(missing=missing):
                with self.assertRaises(resolver_module.ResolverError):
                    resolver_module.wait_for_binding(
                        lambda _timeout, snapshot=snapshot: snapshot,
                        task_uid=UID, source_head_oid=publication["source_head_oid"],
                        scope_base_oid=SCOPE, repository=publication["repository"],
                        pr_number=1, planner_config_sha256=CONFIG,
                        clock=FakeClock().clock, sleep=lambda _duration: None,
                    )

    def test_unstable_snapshots_use_bounded_three_round_schedule(self):
        clock = FakeClock()
        calls = []
        snapshots = [{"complete": False}] * 6
        def read(timeout):
            calls.append(timeout)
            return snapshots[len(calls) - 1]
        with self.assertRaisesRegex(resolver_module.ResolverError, "NETWORK_UNCERTAIN"):
            resolver_module.wait_for_binding(
                read, task_uid=UID, source_head_oid="a" * 40, scope_base_oid=SCOPE,
                repository="eng-cc/oasis7", pr_number=1, planner_config_sha256=CONFIG,
                clock=clock.clock, sleep=clock.sleep,
            )
        self.assertEqual([5.0] * 6, calls)
        self.assertEqual([2.0, 5.0], clock.sleeps)

    def test_task_or_binding_identity_mismatch_is_rejected(self):
        publication, _projection = make_publication(600)
        _value, body = publication_module.prepare(
            task_uid=UID, source_head_oid=publication["source_head_oid"],
            scope_base_oid=SCOPE, projection_digest=publication["projection_digest"],
        )
        binding = publication_module.build_publication_binding(
            publication, 1, f"https://github.com/{publication['repository']}/pull/1",
        )
        binding["pr_number"] = 2
        with self.assertRaises(resolver_module.ResolverError):
            resolver_module.resolve(
                body, task_uid=UID, source_head_oid=publication["source_head_oid"],
                scope_base_oid=SCOPE, publication=publication, binding=binding,
                repository=publication["repository"], pr_number=1,
                planner_config_sha256=CONFIG,
            )

    def test_verified_legacy_b64_projection_is_replaced_by_one_v2_marker(self):
        _contract, marker = publication_module.prepare(
            task_uid=UID, source_head_oid="a" * 40, scope_base_oid=SCOPE,
            projection_digest=digest({"projection": "fixture"}),
        )
        old = "Task: " + UID + "\nRefs #1\n\n<!-- oasis7-impact-projection-b64: abc123== -->"
        updated = publication_module.replace_projection_marker(
            old, marker, legacy_projection_b64="abc123==",
        )
        self.assertNotIn("oasis7-impact-projection-b64", updated)
        self.assertEqual(1, updated.count("oasis7-ci-impact-publication:v2"))
        with self.assertRaises(publication_module.PublicationError):
            publication_module.replace_projection_marker(
                old, marker, legacy_projection_b64="different",
            )


if __name__ == "__main__":
    unittest.main()
