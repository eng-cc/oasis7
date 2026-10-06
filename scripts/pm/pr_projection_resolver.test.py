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
