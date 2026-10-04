#!/usr/bin/env python3
import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).parent


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / f"{name}.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ProjectionCompatibilityTests(unittest.TestCase):
    def test_v1_marker_is_read_as_explicit_legacy_result(self):
        resolver = load("pr_projection_resolver")
        result = resolver.resolve("<!-- oasis7-ci-impact-publication:v1 -->\nlegacy")
        self.assertEqual({"protocol", "legacy", "status", "upgrade_required"}, set(result))
        self.assertTrue(result["legacy"])
        self.assertEqual("legacy-read", result["status"])

    def test_v2_marker_still_requires_current_identity_and_contract(self):
        contract = load("projection_publication_contract")
        resolver = load("pr_projection_resolver")
        value = contract.build_contract(
            task_uid="task_" + "1" * 32,
            source_head_oid="a" * 40,
            scope_base_oid="b" * 40,
            projection_digest="sha256:" + "c" * 64,
            revision=3,
        )
        body = contract.encode_marker(value)
        resolved = resolver.resolve(
            body,
            task_uid=value["task_uid"],
            source_head_oid=value["source_head_oid"],
            scope_base_oid=value["scope_base_oid"],
        )
        self.assertEqual(3, resolved["revision"])

    def test_current_v2_resolution_requires_and_returns_task_full_leaf(self):
        contract = load("projection_publication_contract")
        publication = load("pr_projection_publication")
        resolver = load("pr_projection_resolver")
        task_uid = "task_" + "1" * 32
        head = "a" * 40
        scope = "b" * 40
        config = "sha256:" + "c" * 64
        planner = {
            "schema": "oasis7-required-plan-v1",
            "planner_config_sha256": config,
            "scope": "full",
            "selected_capabilities": [],
            "test_profile": "required",
            "declared_tests": ["required-baseline"],
        }
        leaf = {
            "schema": "oasis7-workflow-impact-projection/v2",
            "task_uid": task_uid,
            "source_head_oid": head,
            "scope_base_oid": scope,
            "changed_paths": [],
            "changed_paths_digest": contract.digest([]),
            "change_class": "unknown",
            "manual_roles": [],
            "domain_role": None,
            "test_profile": "required",
            "declared_tests": ["required-baseline"],
            "consumed_contracts": [],
            "public_semantics": [],
            "affected_consumers": [],
            "closure_status": {"status": "incomplete", "reason": None, "evidence": []},
            "ci_scope": "full",
            "ci_capabilities": [],
            "ci_reasons": [],
            "review_roles": ["qa_engineer"],
            "ordered_role_ids": ["qa_engineer"],
            "review_scope": {},
            "review_escalated": False,
            "review_reasons": [],
            "planner_config_sha256": config,
            "planner_identity": planner,
            "planner_digest": contract.digest(planner),
            "verification_affected": True,
        }
        leaf["projection_digest"] = contract.digest(leaf)
        task_publication = publication.build_task_publication(
            repository="eng-cc/oasis7", repository_id=7, task_uid=task_uid,
            bootstrap_epoch=1, source_repository_id=7, source_ref="feature/current",
            target_ref="main", source_head_oid=head, source_scope_oid=scope,
            planner_authority_oid="d" * 40, planner_config_sha256=config,
            policy_digest=leaf["planner_digest"],
            projection_digest=leaf["projection_digest"],
            workflow_impact_projection=leaf,
        )
        _pr_contract, body = publication.prepare(
            task_uid=task_uid, source_head_oid=head, scope_base_oid=scope,
            projection_digest=leaf["projection_digest"],
        )
        binding = publication.build_publication_binding(
            task_publication, 9, "https://github.com/eng-cc/oasis7/pull/9",
        )
        resolved = resolver.resolve(
            body, task_uid=task_uid, source_head_oid=head, scope_base_oid=scope,
            publication=task_publication, binding=binding,
            repository="eng-cc/oasis7", pr_number=9,
            planner_config_sha256=config, required_protocol="v2",
            live={"repository": "eng-cc/oasis7", "pr_number": 9,
                  "repository_id": 7, "source_repository_id": 7,
                  "source_ref": "feature/current", "target_ref": "main",
                  "head_oid": head, "state": "open", "merged": False,
                  "publication_id": task_publication["publication_id"]},
        )
        self.assertEqual(leaf, resolved["workflow_impact_projection"])

    def test_v1_task_publication_cannot_satisfy_current_v2_resolution(self):
        contract = load("projection_publication_contract")
        publication = load("pr_projection_publication")
        resolver = load("pr_projection_resolver")
        task_uid = "task_" + "1" * 32
        head = "a" * 40
        scope = "b" * 40
        config = "sha256:" + "c" * 64
        leaf_digest = contract.digest({"historical": "digest-only"})
        task_publication = publication.build_task_publication(
            repository="eng-cc/oasis7", repository_id=7, task_uid=task_uid,
            bootstrap_epoch=1, source_repository_id=7, source_ref="feature/legacy",
            target_ref="main", source_head_oid=head, source_scope_oid=scope,
            planner_authority_oid="d" * 40, planner_config_sha256=config,
            policy_digest=contract.digest({"policy": "legacy"}),
            projection_digest=leaf_digest,
        )
        _pr_contract, body = publication.prepare(
            task_uid=task_uid, source_head_oid=head, scope_base_oid=scope,
            projection_digest=leaf_digest,
        )
        binding = publication.build_publication_binding(
            task_publication, 9, "https://github.com/eng-cc/oasis7/pull/9",
        )
        with self.assertRaisesRegex(resolver.ResolverError, "full projection"):
            resolver.resolve(
                body, task_uid=task_uid, source_head_oid=head, scope_base_oid=scope,
                publication=task_publication, binding=binding,
                repository="eng-cc/oasis7", pr_number=9,
                planner_config_sha256=config, required_protocol="v2",
            )

    def test_non_text_body_fails_closed(self):
        resolver = load("pr_projection_resolver")
        with self.assertRaises(resolver.ResolverError):
            resolver.resolve(None)

    def test_oversized_body_fails_before_protocol_detection(self):
        resolver = load("pr_projection_resolver")
        with self.assertRaisesRegex(resolver.ResolverError, "60KiB"):
            resolver.resolve("<!-- oasis7-ci-impact-publication:v1 -->\n" + "x" * (60 * 1024))

    def test_v1_marker_must_be_unique_and_unmixed(self):
        resolver = load("pr_projection_resolver")
        marker = "<!-- oasis7-ci-impact-publication:v1 -->"
        with self.assertRaises(resolver.ResolverError):
            resolver.resolve(marker)
        with self.assertRaises(resolver.ResolverError):
            resolver.resolve("prefix\n" + marker + "\nlegacy")
        with self.assertRaises(resolver.ResolverError):
            resolver.resolve(marker + "\nlegacy\n" + marker)

    def test_v1_and_v2_markers_cannot_be_mixed(self):
        resolver = load("pr_projection_resolver")
        with self.assertRaisesRegex(resolver.ResolverError, "mixed"):
            resolver.resolve(
                "<!-- oasis7-ci-impact-publication:v1 -->\nlegacy\n"
                "<!-- oasis7-ci-impact-publication:v2 -->"
            )

    def test_publication_default_is_revision_three(self):
        publication = load("pr_projection_publication")
        value, _ = publication.prepare(
            task_uid="task_" + "1" * 32,
            source_head_oid="a" * 40,
            scope_base_oid="b" * 40,
            projection_digest="sha256:" + "c" * 64,
        )
        self.assertEqual(3, value["revision"])


if __name__ == "__main__":
    unittest.main()
