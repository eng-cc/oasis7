#!/usr/bin/env python3
"""C1 RED contract for complete fixtures and identity/version boundaries.

This suite intentionally exercises the public, read-only C1 validation seams
before their implementation is extended.  The fixture is the executable
complete sample; the governance snippets remain explanatory excerpts.
"""

from copy import deepcopy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest


HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "loop-change-c1-complete.json"


def _load(name, path):
    if str(HERE) not in sys.path:
        sys.path.insert(0, str(HERE))
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TRACE = _load("loop_traceability_c1_red", HERE / "loop_traceability.py")
CONTRACTS = _load("loop_contracts_c1_red", HERE / "loop_contracts.py")


def _canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _refresh_digest(record):
    record = deepcopy(record)
    record["coordination_ref"]["record_digest"] = ""
    record["coordination_ref"]["record_digest"] = (
        "sha256:" + hashlib.sha256(_canonical(record).encode("utf-8")).hexdigest()
    )
    return record


class C1RedTests(unittest.TestCase):
    def setUp(self):
        self.record = json.loads(FIXTURE.read_text(encoding="utf-8"))

    def test_complete_fixture_is_valid_and_read_only(self):
        before = deepcopy(self.record)
        result = TRACE.validate_record(self.record)
        self.assertEqual(
            result,
            {"status": "passed", "blockers": [], "compatibility_mode": "current"},
            result,
        )
        self.assertEqual(self.record, before)

    def test_wrong_repository_has_field_local_diagnostic(self):
        record = deepcopy(self.record)
        record["required_obligations"][0]["trace"]["upstream_refs"][0]["repository"] = "foreign/repo"
        result = TRACE.validate_record(_refresh_digest(record))
        self.assertEqual(result.get("status"), "blocked", result)
        blockers = "\n".join(result.get("blockers", []))
        self.assertIn("trace.upstream_refs[0]", blockers)
        self.assertIn(record["task_uid"], blockers)
        self.assertIn("repository", blockers)
        diagnostics = result.get("diagnostics")
        self.assertIsInstance(diagnostics, list, result)
        self.assertEqual(diagnostics[0]["code"], "trace-ref-unresolved")
        self.assertEqual(diagnostics[0]["task_uid"], record["task_uid"])
        self.assertEqual(diagnostics[0]["obligation_id"], "C1-identity")
        self.assertEqual(diagnostics[0]["field"], "trace.upstream_refs[0]")
        self.assertTrue(diagnostics[0]["repair_hint"], diagnostics)

    def test_diagnostics_expose_the_closed_c1_error_class_vocabulary(self):
        task_uid = self.record["task_uid"]
        cases = (
            (
                "trace-ref-unresolved: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: trace.upstream_refs[0] repository mismatch",
                "invalid_reference",
            ),
            (
                "trace-revision-type: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: trace.upstream_refs[0] revision must be a positive integer",
                "revision_type_mismatch",
            ),
            (
                "trace-identity-oid-placement: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: trace.upstream_refs[0] source_commit belongs to the source snapshot",
                "source_snapshot_mismatch",
            ),
            (
                "trace-legacy-upgrade-required: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: legacy obligations require explicit traces",
                "legacy_compatibility",
            ),
            (
                "live-authority-read: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: authority reader readback unavailable",
                "authority_unavailable",
            ),
            (
                "validation-error: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: composition evidence missing",
                "projection_absent",
            ),
            (
                "validation-error: record {task} change @ eng-cc/oasis7#3730/comment/1 "
                "obligation C1-identity: required_obligations must be a non-empty list",
                "empty_projection",
            ),
        )
        for template, expected_class in cases:
            with self.subTest(expected_class=expected_class):
                diagnostic = TRACE._diagnostic(template.format(task=task_uid))
                self.assertEqual(diagnostic["error_class"], expected_class, diagnostic)
                self.assertTrue(diagnostic["code"], diagnostic)
                self.assertEqual(diagnostic["message"].split(":", 1)[0], diagnostic["code"])
                self.assertEqual(diagnostic["task_uid"], task_uid)
                self.assertEqual(diagnostic["obligation_id"], "C1-identity")
                self.assertTrue(diagnostic["repair_hint"], diagnostic)

    def test_error_class_projection_preserves_existing_diagnostic_fields(self):
        message = (
            f"trace-ref-unresolved: record {self.record['task_uid']} change @ "
            "eng-cc/oasis7#3730/comment/1 obligation C1-identity: "
            "trace.upstream_refs[0] repository mismatch; repair: rebind the relation"
        )
        diagnostic = TRACE._diagnostic(message)
        self.assertEqual(diagnostic["code"], "trace-ref-unresolved")
        self.assertEqual(diagnostic["message"], message)
        self.assertEqual(diagnostic["task_uid"], self.record["task_uid"])
        self.assertEqual(diagnostic["obligation_id"], "C1-identity")
        self.assertEqual(diagnostic["field"], "trace.upstream_refs[0]")
        self.assertTrue(diagnostic["repair_hint"], diagnostic)
        self.assertEqual(diagnostic["error_class"], "invalid_reference")

    def test_legacy_fixture_remains_explicitly_readable(self):
        legacy = deepcopy(self.record)
        for obligation in legacy["required_obligations"]:
            obligation.pop("applicability", None)
            obligation.pop("trace", None)
        result = TRACE.validate_record(_refresh_digest(legacy))
        self.assertEqual(
            result,
            {"status": "passed", "blockers": [], "compatibility_mode": "legacy"},
            result,
        )

    def test_typed_upstream_rejects_misplaced_source_oid(self):
        record = deepcopy(self.record)
        record["required_obligations"][0]["trace"]["upstream_refs"][0]["source_commit"] = "a" * 40
        result = TRACE.validate_record(_refresh_digest(record))
        self.assertEqual(result.get("status"), "blocked", result)
        self.assertIn("source_commit", "\n".join(result.get("blockers", [])))

    def test_typed_upstream_rejects_non_integer_contract_revision(self):
        record = deepcopy(self.record)
        record["required_obligations"][0]["trace"]["upstream_refs"][0]["revision"] = "a" * 40
        result = TRACE.validate_record(_refresh_digest(record))
        self.assertEqual(result.get("status"), "blocked", result)
        blockers = "\n".join(result.get("blockers", []))
        self.assertIn("revision", blockers)
        self.assertIn("trace.upstream_refs[0]", blockers)

    def test_bound_consumed_clause_rejects_misplaced_source_oid(self):
        reference = deepcopy(self.record["consumed_clause_refs"][0])
        reference["source_commit"] = "b" * 40
        errors = CONTRACTS.consumed_clause_ref_errors(reference, require_identity=True)
        self.assertTrue(
            any("source_commit" in error for error in errors),
            errors,
        )

    def test_hosted_record_rejects_consumed_clause_source_oid(self):
        record = deepcopy(self.record)
        record["consumed_clause_refs"][0]["source_commit"] = "b" * 40
        result = TRACE.validate_record(_refresh_digest(record))
        self.assertEqual(result.get("status"), "blocked", result)
        self.assertIn("source_commit", "\n".join(result.get("blockers", [])))

    def test_hosted_record_rejects_consumed_clause_non_integer_revision(self):
        record = deepcopy(self.record)
        record["consumed_clause_refs"][0]["revision"] = "1"
        result = TRACE.validate_record(_refresh_digest(record))
        self.assertEqual(result.get("status"), "blocked", result)
        blockers = "\n".join(result.get("blockers", []))
        self.assertIn("consumed_clause_refs[0]", blockers)
        self.assertIn("revision", blockers)
        diagnostic = result["diagnostics"][0]
        self.assertEqual(diagnostic["error_class"], "revision_type_mismatch")
        self.assertEqual(diagnostic["field"], "consumed_clause_refs[0].revision")
        self.assertEqual(diagnostic["task_uid"], record["task_uid"])

    def test_hosted_record_rejects_consumed_clause_publication_repository_drift(self):
        for repository in (None, "foreign/repo"):
            with self.subTest(repository=repository):
                record = deepcopy(self.record)
                publication = record["consumed_clause_refs"][0]["publication_ref"]
                if repository is None:
                    publication.pop("repository")
                else:
                    publication["repository"] = repository
                result = TRACE.validate_record(_refresh_digest(record))
                self.assertEqual(result.get("status"), "blocked", result)
                diagnostic = result["diagnostics"][0]
                self.assertEqual(diagnostic["error_class"], "invalid_reference")
                self.assertEqual(
                    diagnostic["field"],
                    "consumed_clause_refs[0].publication_ref.repository",
                )

    def test_hosted_record_rejects_malformed_consumed_clause_digests(self):
        for field in ("source_digest", "content_digest"):
            with self.subTest(field=field):
                record = deepcopy(self.record)
                record["consumed_clause_refs"][0][field] = "not-a-digest"
                result = TRACE.validate_record(_refresh_digest(record))
                self.assertEqual(result.get("status"), "blocked", result)
                diagnostic = result["diagnostics"][0]
                self.assertEqual(diagnostic["error_class"], "invalid_reference")
                self.assertEqual(
                    diagnostic["field"], f"consumed_clause_refs[0].{field}"
                )

    def test_schema_keeps_typed_revision_integer_and_oid_out_of_clause_refs(self):
        schema = json.loads(
            (HERE / "schemas" / "loop-change.schema.json").read_text(encoding="utf-8")
        )
        for name in ("typed_upstream_ref", "system_design_ref"):
            definition = schema["$defs"][name]
            properties = definition["allOf"][0]["properties"]
            self.assertEqual(properties["revision"], {"type": "integer", "minimum": 1})
            self.assertIn("not", definition)
            self.assertIn("anyOf", definition["not"])
        self.assertIn("not", schema["$defs"]["bound_path_ref"])

    def test_na_evidence_rejects_oid_or_mixed_locator_identity(self):
        path_evidence = {
            "repository": "eng-cc/oasis7",
            "path": "doc/engineering/workflow/source-of-truth.md",
            "fragment": "traceability-record-contract",
            "source_commit": "a" * 40,
        }
        with self.assertRaises(TRACE.TraceabilityError):
            TRACE._validate_na_evidence_locator(path_evidence, "trace.system_design.evidence_ref")

        mixed_evidence = {
            "repository": "eng-cc/oasis7",
            "path": "doc/engineering/workflow/source-of-truth.md",
            "fragment": "traceability-record-contract",
            "issue_number": 3730,
            "comment_id": 999001,
        }
        with self.assertRaises(TRACE.TraceabilityError):
            TRACE._validate_na_evidence_locator(mixed_evidence, "trace.system_design.evidence_ref")

    def test_complete_fixture_reads_back_authority_and_frozen_fragments(self):
        source_commit = self.record["coordination_ref"]["source_commit"]
        expected_body = {
            "marker": self.record["marker"],
            "schema": self.record["schema"],
            "task_uid": self.record["task_uid"],
            "change_id": self.record["change_id"],
            "record_digest": self.record["coordination_ref"]["record_digest"],
            "record": self.record,
        }

        class FixtureAuthority:
            reader_kind = "fixture_authority"

            def __call__(self, reference):
                issue_number = reference["issue_number"]
                return {
                    "reader_kind": self.reader_kind,
                    "repository": "eng-cc/oasis7",
                    "issue": {
                        "number": issue_number,
                        "html_url": f"https://github.com/eng-cc/oasis7/issues/{issue_number}",
                        "body": f"<!-- oasis7-pm-task -->\ntask_uid: {self_task_uid}\n",
                    },
                    "comment": {
                        "id": reference["comment_id"],
                        "issue_url": f"https://api.github.com/repos/eng-cc/oasis7/issues/{issue_number}",
                        "body": _canonical(expected_body),
                        "user": {"login": "coordinator"},
                        "created_at": "2026-09-11T00:00:00Z",
                    },
                }

        self_task_uid = self.record["task_uid"]
        binding = {
            "schema": "oasis7.loop-task/v1",
            "task_uid": self_task_uid,
            "change_id": self.record["change_id"],
            "coordination_ref": deepcopy(self.record["coordination_ref"]),
            "policy_commit": source_commit,
            "policy_digest": "sha256:" + "4" * 64,
        }
        result = TRACE.validate_leaf(
            self.record,
            binding,
            authority_reader=FixtureAuthority(),
            contract_reader=TRACE.ImmutableSourceReader(HERE.parent.parent, source_commit),
            source_commit=source_commit,
        )
        self.assertEqual(result.get("status"), "passed", result)

    def test_document_excerpt_marker_is_not_the_complete_fixture(self):
        document = (
            HERE.parent.parent
            / "doc/engineering/doc-governance/cross-layer-requirements-traceability.design.md"
        ).read_text(encoding="utf-8")
        self.assertIn("字段节选（不是完整 oasis7.loop-change/v1 记录", document)
        self.assertEqual(self.record["schema"], "oasis7.loop-change/v1")
        self.assertIn("required_obligations", self.record)


if __name__ == "__main__":
    unittest.main(verbosity=2)
