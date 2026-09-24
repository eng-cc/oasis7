#!/usr/bin/env python3
"""Focused fake-adapter coverage for C1 ordering, recovery, and resolution."""
from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

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
                     repository: str = "eng-cc/oasis7"):
    head = head or f"{index + 1:040x}"
    branch = branch or f"feature/c1-{index}"
    projection_digest = digest({"target": index})
    publication = publication_module.build_task_publication(
        repository=repository, repository_id=7, task_uid=UID,
        bootstrap_epoch=1, source_repository_id=7, source_ref=branch,
        target_ref="main", source_head_oid=head, source_scope_oid=SCOPE,
        planner_authority_oid=AUTHORITY, planner_config_sha256=CONFIG,
        policy_digest=digest({"policy": "test"}), projection_digest=projection_digest,
    )
    projection = {
        "task_uid": UID, "source_head_oid": head, "scope_base_oid": SCOPE,
        "planner_config_sha256": CONFIG, "projection_digest": projection_digest,
        "consumed_contracts": [],
    }
    return publication, projection


class FakeAdapter:
    def __init__(self, publication, projection, *, initial_head=None, initial_pr=None):
        self.publications = []
        self.bindings = []
        self.pr_binding = None
        self.events = []
        self.source_ref = initial_head
        self.projection = projection
        self.prs = []
        if initial_pr:
            self.prs.append(copy.deepcopy(initial_pr))
            self.pr_binding = {"task_uid": UID, "pr_number": initial_pr["number"]}

    def find_task_publications(self, pub_id):
        return {"complete": True, "publications": [p for p in self.publications if p["publication_id"] == pub_id]}

    def publish_task_intent(self, value):
        self.events.append("task-intent")
        self.publications.append(copy.deepcopy(value))

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
        return copy.deepcopy(self.pr_binding or {"task_uid": task_uid, "pr_number": None})

    def record_pr(self, task_uid, number, publication_id):
        self.events.append("record-pr")
        self.pr_binding = {"task_uid": task_uid, "pr_number": number}

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


class FakeClock:
    def __init__(self):
        self.now = 0.0
        self.sleeps = []

    def clock(self):
        return self.now

    def sleep(self, duration):
        self.sleeps.append(duration)
        self.now += duration


class PublicationMatrixTests(unittest.TestCase):
    def journal(self, temp, publication):
        return journal_module.open_journal(
            temp, publication["repository"], publication["source_ref"],
            publication["publication_id"], task_uid=publication["task_uid"],
            source_head_oid=publication["source_head_oid"],
            scope_base_oid=publication["source_scope_oid"],
            projection_digest=publication["projection_digest"],
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

        argv = command.call_args.args[0]
        self.assertIn("record-pr", argv)
        self.assertIn("--repo", argv)
        repo_index = argv.index("--repo")
        self.assertEqual("example/oasis7", argv[repo_index + 1])
        self.assertEqual("https://github.com/example/oasis7/pull/999",
                         argv[argv.index("--pr-url") + 1])

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
