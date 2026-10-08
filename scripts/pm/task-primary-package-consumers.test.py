#!/usr/bin/env python3
"""Current completion identity through real projection/review/CI/router consumers.

Git and Cargo are real; only GitHub service responses are simulated.
"""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


fixture = load("primary_consumer_fixture", "task-primary-package.test.py")
projection = load("primary_consumer_projection", "workflow-impact-projection.py")
review = load("primary_consumer_review", "review-plan.py")
identity = load("primary_consumer_identity", "ci_ready_receipt_identity.py")
router = load("primary_consumer_router", "workflow-next.py")
snapshot = load("primary_consumer_snapshot", "bootstrap-task-snapshot.py")
reuse_fixture = load("primary_consumer_reuse_fixture", "review-identity-v2.test.py")


class ConsumerTests(unittest.TestCase):
    setUp = fixture.CompletionTests.setUp
    git = fixture.CompletionTests.git
    write = fixture.CompletionTests.write
    save = fixture.CompletionTests.save
    current = fixture.CompletionTests.current
    project_readback = fixture.CompletionTests.project_readback
    permission_readback = fixture.CompletionTests.permission_readback
    github = fixture.CompletionTests.github
    external_command = fixture.CompletionTests.external_command
    complete = fixture.CompletionTests.complete
    policy_authority_response = fixture.CompletionTests.policy_authority_response

    def completed(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.complete()
        for module in (projection, review, identity, router, snapshot):
            p = patch.object(module.primary_contract, "_github", side_effect=self.github)
            p.start()
            self.addCleanup(p.stop)
        return fixture.contract.completion_reference(self.current())

    def test_completed_record_keeps_independent_identity_through_Project_projection(self):
        self.completed()
        record = self.current()
        before = json.dumps(record, sort_keys=True)
        task = fixture.facade.task_from_record(fixture.UID, record)
        self.assertEqual(fixture.contract.immutable_identity(task),
                         fixture.contract.immutable_identity(record))
        sync = fixture.facade.load_sync_module()
        values = sync.project_field_values(task)
        self.assertEqual(values["Primary Package"], "alpha")
        self.assertEqual(values["Canonical Worktree"], str(self.root))
        self.assertEqual(json.dumps(record, sort_keys=True), before)
        body = fixture.facade.issue_body(task)
        self.assertNotIn("- project_item_id:", body)
        self.assertNotIn("- repository:", body)

    def test_completed_identity_rejects_every_independent_field_mismatch(self):
        self.completed()
        record = self.current()
        for field in fixture.contract.IDENTITY_FIELDS:
            with self.subTest(field=field):
                altered = dict(record)
                altered[field] = ["different acceptance"] if field == "acceptance" else (
                    2 if field in {"issue_number", "bootstrap_epoch"} else "different")
                with self.assertRaisesRegex(ValueError, "immutable Task identity differs"):
                    fixture.contract.validate_completion(altered)
        lifecycle = {**record, "status": "verification", "pr_url": "https://github.com/eng-cc/oasis7/pull/99"}
        self.assertIsNotNone(fixture.contract.validate_completion(lifecycle))
        legacy = dict(record); legacy.pop("bootstrap_epoch")
        self.assertIsNotNone(fixture.contract.validate_completion(legacy))

    def input(self, refs):
        return {"task_uid": fixture.UID, "source_head_oid": self.git("rev-parse", "HEAD"),
                "scope_base_oid": self.base, "changed_paths": ["crates/alpha/src/lib.rs"],
                "change_class": "mixed", "manual_roles": ["runtime_engineer"], "domain_role": None,
                "test_profile": "required", "declared_tests": ["required_gate_baseline"],
                "consumed_contracts": refs, "public_semantics": [], "affected_consumers": [],
                "closure_status": {"status": "complete", "reason": "verified", "evidence": [{
                    "path": "Cargo.toml", "sha256": "sha256:" + hashlib.sha256((self.root / "Cargo.toml").read_bytes()).hexdigest()}]}}

    def test_projection_builder_and_verified_loader_reject_stale_contracts(self):
        reference = self.completed()
        good = projection.build_projection(self.root, self.input([reference]))
        path = self.root / "projection.json"
        path.write_text(json.dumps(good))
        self.assertEqual(good, projection.load_verified_projection(path, repo_root=self.root))
        for refs in ([], [reference, reference], [{**reference, "comment_id": 100}]):
            with self.subTest(refs=refs):
                with self.assertRaises(projection.ProjectionError):
                    projection.build_projection(self.root, self.input(refs))
                forged = {**good, "consumed_contracts": refs}
                forged["projection_digest"] = projection.canonical_digest({k: v for k, v in forged.items() if k != "projection_digest"})
                path.write_text(json.dumps(forged))
                with self.assertRaises(projection.ProjectionError):
                    projection.load_verified_projection(path, repo_root=self.root)
        path.write_text(json.dumps(good))
        self.comments[0]["updated_at"] = "edited"
        with self.assertRaises(projection.ProjectionError):
            projection.load_verified_projection(path, repo_root=self.root)

    def test_rootless_verified_loader_recognizes_v2_even_with_forged_type(self):
        reference = self.completed()
        self.assertEqual(fixture.contract.SCOPE_SCHEMA, reference["schema"])
        good = projection.build_projection(self.root, self.input([reference]))
        path = self.root / "rootless.json"
        path.write_text(json.dumps(good))
        self.assertEqual(good, projection.load_verified_projection(path))
        for refs in ([{**reference, "type": "unrelated-contract"}],
                     [{k: v for k, v in reference.items() if k != "type"}],
                     [{**reference, "schema": "unknown-completion/v99"}],
                     [reference, reference]):
            forged = {**good, "consumed_contracts": refs}
            forged["projection_digest"] = projection.canonical_digest({
                k: v for k, v in forged.items() if k != "projection_digest"})
            path.write_text(json.dumps(forged))
            with self.subTest(refs=refs), self.assertRaises(projection.ProjectionError):
                projection.load_verified_projection(path)

    def test_rootless_review_reuse_requires_current_authority_for_v2(self):
        reference = self.completed()
        source = identity.source_review_identity(**reuse_fixture.source_fields())
        plan = reuse_fixture.ordinary_plan(source)
        receipt = reuse_fixture.ordinary_receipt(
            impact_projection_digest=plan["impact_projection_digest"],
            impact_projection_planner_digest=plan["impact_projection_planner_digest"])
        self.assertTrue(identity.can_reuse_source_review(plan, receipt))
        for refs in ([reference], [{**reference, "type": "unrelated-contract"}],
                     [{k: v for k, v in reference.items() if k != "type"}],
                     [reference, reference]):
            candidate = {**plan, "impact_projection": {
                **plan["impact_projection"], "consumed_contracts": refs}}
            value = candidate["impact_projection"]
            value["projection_digest"] = projection.canonical_digest({
                k: v for k, v in value.items() if k != "projection_digest"})
            candidate["impact_projection_digest"] = value["projection_digest"]
            current_receipt = {**receipt, "impact_projection_digest": value["projection_digest"]}
            with self.subTest(refs=refs):
                self.assertFalse(identity.can_reuse_source_review(candidate, current_receipt))

    def test_review_source_input_rejects_omitted_duplicate_and_replacement(self):
        reference = self.completed()
        current = self.current()
        current["pr_number"] = 12
        self.record = current
        self.save()
        for relative in (".agents/roles/qa_engineer.md", "doc/engineering/workflow/source-of-truth.md",
                         ".agents/skills/requesting-repo-owned-review/SKILL.md"):
            self.write(relative, (fixture.ROOT / relative).read_text())
        good = projection.build_projection(self.root, self.input([reference]))
        source, applicability = review.derived_source_review_input(self.root,
            task_uid=fixture.UID, head=good["source_head_oid"], comparison_oid=self.base,
            roles=["qa_engineer"], impact_projection=good, bootstrap_epoch=1)
        self.assertEqual(good["projection_digest"].removeprefix("sha256:"), source["input_contract_digest"])
        self.assertTrue(applicability["verified"])
        for refs in ([], [reference, reference], [{**reference, "body_sha256": "sha256:" + "0" * 64}]):
            with self.subTest(refs=refs), self.assertRaisesRegex(review.ContractError, "completion"):
                review.derived_source_review_input(self.root, task_uid=fixture.UID,
                    head=self.git("rev-parse", "HEAD"), comparison_oid=self.base,
                    roles=["qa_engineer"], impact_projection=self.input(refs), bootstrap_epoch=1)

    def test_ci_target_relations_validate_server_and_keep_metadata_pathless(self):
        reference = self.completed()
        value = self.input([reference])
        paths, unmapped = identity._target_relation_paths(value, root=self.root, source_head=value["source_head_oid"])
        self.assertEqual(["crates/alpha/src/lib.rs", "Cargo.toml"], paths)
        self.assertFalse(unmapped)
        source = identity.source_review_identity(task_uid=fixture.UID, bootstrap_epoch=1,
            repository=fixture.REPO, pr_number=12, source_head_oid=value["source_head_oid"],
            source_scope_oid=self.base, changed_paths_digest="1" * 64,
            ordered_role_ids=["qa_engineer"], role_contract_digest="2" * 64,
            review_policy_digest="3" * 64, input_contract_digest="4" * 64)
        self.git("switch", "-qc", "target-advance", self.base)
        self.write("unrelated.txt", "unrelated target advance\n")
        self.git("add", "unrelated.txt")
        self.git("commit", "-qm", "unrelated")
        plan = {"source_review_identity": source, "impact_projection": value}
        self.assertFalse(identity._target_advance_requires_strict(plan, root=self.root,
            current_target_oid=self.git("rev-parse", "HEAD")))
        self.write("crates/alpha/src/lib.rs", "pub fn target_changed() {}\n")
        self.git("add", "crates/alpha/src/lib.rs")
        self.git("commit", "-qm", "related")
        self.assertTrue(identity._target_advance_requires_strict(plan, root=self.root,
            current_target_oid=self.git("rev-parse", "HEAD")))
        for refs in ([], [reference, reference], [{**reference, "comment_id": 100}],
                     [{**reference, "type": "unrelated-contract"}]):
            with self.subTest(refs=refs), self.assertRaises(ValueError):
                identity._target_relation_paths({**value, "consumed_contracts": refs}, root=self.root, source_head=value["source_head_oid"])
        self.comments[0]["updated_at"] = "edited"
        with self.assertRaises(ValueError):
            identity._target_relation_paths(value, root=self.root, source_head=value["source_head_oid"])

    def test_real_loop_completion_verified_projection_uses_bounded_reader(self):
        original = fixture.contract.validate_current_completion
        verified = []
        for module in (projection, identity):
            p = patch.object(module.primary_contract, "_github", side_effect=self.github)
            p.start()
            self.addCleanup(p.stop)

        def consumer(root, task):
            original(root, task)
            if fixture.contract.FIELD not in self.current():
                return
            reference = fixture.contract.completion_reference(task)
            value = projection.build_projection(self.root, self.input([reference]))
            path = self.root / "loop-projection.json"
            path.write_text(json.dumps(value))
            self.assertEqual(value, projection.load_verified_projection(path, repo_root=self.root))
            _, unmapped = identity._target_relation_paths(value, root=self.root,
                source_head=value["source_head_oid"])
            self.assertFalse(unmapped)
            verified.append(reference)

        # The shared fixture runs the real pinned loop gate, Git and Cargo;
        # its explicit full-admission trap remains active during these consumers.
        with patch.object(fixture.contract, "validate_current_completion", side_effect=consumer):
            fixture.CompletionTests.test_real_canonical_loop_admission_and_nonrecursive_reader(self)
        self.assertTrue(verified)

    def test_router_reports_current_completion_and_rejects_server_drift(self):
        reference = self.completed()
        args = ["workflow-next.py", "--repo-root", str(self.root), "--task-uid", fixture.UID, "--json"]
        for stale in (False, True):
            if stale:
                self.comments[0]["updated_at"] = "edited"
            out = io.StringIO()
            with patch.object(sys, "argv", args), contextlib.redirect_stdout(out):
                router.main()
            value = json.loads(out.getvalue())
            if stale:
                self.assertTrue(any("primary completion identity" in b for b in value["blockers"]))
            else:
                self.assertEqual(reference, value["primary_package_completion"])


if __name__ == "__main__":
    unittest.main()
