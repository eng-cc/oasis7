#!/usr/bin/env python3
"""Focused contract tests for the proposed CI applicability reader/evaluator."""

import unittest

import ci_evidence_applicability as applicability
import ci_ready_receipt_identity as receipt_identity
import projection_publication_contract as publication


UID = "task_12345678901234567890123456789012"
REPOSITORY = "eng-cc/oasis7"
HEAD = "a" * 40
TARGET = "b" * 40
DIGEST = f"sha256:{'c' * 64}"
INPUT_DIGEST = f"sha256:{'d' * 64}"
CAPABILITY = "input-scope-reuse/v1"
PLAN_SCHEMA = "oasis7-required-plan-v2"


def source_plan():
    return {
        "schema": PLAN_SCHEMA,
        "required_capabilities": [CAPABILITY],
        "repository": REPOSITORY,
        "task_uid": UID,
        "pr_number": 7,
        "source_head_oid": HEAD,
        "review_applicability_digest": DIGEST,
        "required_test_units": ["unit-a"],
        "required_review_roles": ["runtime_engineer"],
    }


def target_snapshot(*, input_digest=INPUT_DIGEST, review_digest=DIGEST):
    return {
        "repository": REPOSITORY,
        "task_uid": UID,
        "pr_number": 7,
        "source_head_oid": HEAD,
        "target_oid": TARGET,
        "review_applicability_digest": review_digest,
        "required_test_units": ["unit-a"],
        "required_review_roles": ["runtime_engineer"],
        "input_fingerprints": {"unit-a": input_digest},
    }


def evidence_set(*, test_input_digest=INPUT_DIGEST, review_digest=DIGEST):
    return {
        "reviews": [{
            "role_id": "runtime_engineer",
            "status": "passed",
            "repository": REPOSITORY,
            "task_uid": UID,
            "pr_number": 7,
            "source_head_oid": HEAD,
            "applicability_digest": review_digest,
        }],
        "tests": [{
            "unit_id": "unit-a",
            "status": "passed",
            "input_digest": test_input_digest,
            "repository": REPOSITORY,
            "task_uid": UID,
            "pr_number": 7,
            "source_head_oid": HEAD,
            "run_id": 10,
            "run_attempt": 1,
            "check_app_id": 42,
            "check_run_id": 20,
            "artifact_id": 30,
        }],
    }


def enabled_policy():
    return {"enabled_capabilities": [CAPABILITY], "check_app_id": 42}


def publication_v1():
    value = {
        "schema": "oasis7-ci-publication/v1",
        "repository": REPOSITORY,
        "repository_id": 123,
        "task_uid": UID,
        "bootstrap_epoch": 1,
        "source_repository_id": 123,
        "source_ref": "task/ci-reuse",
        "target_ref": "main",
        "source_head_oid": HEAD,
        "source_scope_oid": TARGET,
        "planner_authority_oid": "e" * 40,
        "planner_config_sha256": f"sha256:{'f' * 64}",
        "policy_digest": f"sha256:{'1' * 64}",
        "projection_digest": f"sha256:{'2' * 64}",
        "projection_required": True,
    }
    value["publication_id"] = publication.digest({
        key: value[key] for key in publication.CI_PUBLICATION_IDENTITY_FIELDS
    })
    return value


def result_status(result, field):
    value = result.to_dict()[field]
    return value["status"] if isinstance(value, dict) else value


class RequiredPlanCapabilityTests(unittest.TestCase):
    def test_legacy_required_plan_is_left_on_legacy_reader_path(self):
        self.assertIsNone(receipt_identity.read_required_plan_capabilities({
            "schema": "oasis7-required-plan-v1",
        }))

    def test_v2_requires_and_recognizes_the_exact_capability(self):
        self.assertEqual(
            (CAPABILITY,),
            receipt_identity.read_required_plan_capabilities({
                "schema": PLAN_SCHEMA,
                "required_capabilities": [CAPABILITY],
            }),
        )

    def test_unsupported_schema_or_capability_fails_closed(self):
        cases = (
            {"schema": "oasis7-required-plan-v3", "required_capabilities": [CAPABILITY]},
            {"schema": PLAN_SCHEMA, "required_capabilities": []},
            {"schema": PLAN_SCHEMA, "required_capabilities": ["future-capability/v1"]},
            {"schema": PLAN_SCHEMA, "required_capabilities": [CAPABILITY, CAPABILITY]},
            {"schema": PLAN_SCHEMA},
        )
        for plan in cases:
            with self.subTest(plan=plan), self.assertRaises(ValueError):
                receipt_identity.read_required_plan_capabilities(plan)

    def test_legacy_schema_cannot_smuggle_new_capability(self):
        with self.assertRaises(ValueError):
            receipt_identity.read_required_plan_capabilities({
                "schema": "oasis7-required-plan-v1",
                "required_capabilities": [CAPABILITY],
            })


class PublicationCompatibilityTests(unittest.TestCase):
    def test_proposed_v1_publication_requires_exact_fields_and_identity_digest(self):
        valid = publication_v1()
        self.assertEqual(valid, publication.validate_ci_publication(valid))

        unknown = {**valid, "ignored_future_field": True}
        missing = {key: value for key, value in valid.items() if key != "source_scope_oid"}
        wrong_identity = {**valid, "source_head_oid": "f" * 40}
        for candidate in (unknown, missing, wrong_identity):
            with self.subTest(candidate=candidate), self.assertRaises(publication.ContractError):
                publication.validate_ci_publication(candidate)

    def test_existing_v2_publication_marker_wire_round_trips_unchanged(self):
        contract = publication.build_contract(
            task_uid=UID,
            source_head_oid=HEAD,
            scope_base_oid=TARGET,
            projection_digest=DIGEST,
        )
        marker = publication.encode_marker(contract)

        self.assertEqual("oasis7-ci-impact-publication/v2", contract["schema"])
        self.assertEqual(contract, publication.decode_marker(marker))
        self.assertEqual(marker, publication.encode_marker(publication.decode_marker(marker)))


class ApplicabilityDecisionTests(unittest.TestCase):
    def evaluate(self, *, plan=None, evidence=None, target=None, policy=None):
        return applicability.evaluate_evidence_applicability(
            source_plan() if plan is None else plan,
            evidence_set() if evidence is None else evidence,
            target_snapshot() if target is None else target,
            enabled_policy() if policy is None else policy,
        )

    def test_reuse_is_disabled_when_policy_omits_capability(self):
        decision = self.evaluate(policy={})

        self.assertEqual("disabled", result_status(decision, "source_review"))
        self.assertEqual("disabled", result_status(decision, "test_evidence"))
        self.assertEqual("disabled", result_status(decision, "merge_readiness"))

    def test_unknown_policy_capability_blocks(self):
        decision = self.evaluate(policy={"enabled_capabilities": ["future-capability/v1"]})

        self.assertEqual("blocked", result_status(decision, "merge_readiness"))
        self.assertTrue(decision.blockers)

    def test_exact_identity_and_provenance_allow_both_evidence_dimensions(self):
        decision = self.evaluate()

        self.assertEqual("reusable", result_status(decision, "source_review"))
        self.assertEqual("reusable", result_status(decision, "test_evidence"))
        self.assertEqual("reusable", result_status(decision, "merge_readiness"))
        self.assertEqual(("unit-a",), tuple(decision.reused_units))

    def test_changed_test_input_revalidates_tests_without_invalidating_review(self):
        decision = self.evaluate(
            target=target_snapshot(input_digest=f"sha256:{'e' * 64}"),
        )

        self.assertEqual("reusable", result_status(decision, "source_review"))
        self.assertEqual("revalidate", result_status(decision, "test_evidence"))
        self.assertEqual("revalidate", result_status(decision, "merge_readiness"))
        self.assertEqual(("unit-a",), tuple(decision.required_test_units))

    def test_review_input_change_does_not_invalidate_unchanged_test_unit(self):
        changed_review_digest = f"sha256:{'e' * 64}"
        decision = self.evaluate(
            target=target_snapshot(review_digest=changed_review_digest),
            evidence=evidence_set(review_digest=DIGEST),
        )

        self.assertEqual("revalidate", result_status(decision, "source_review"))
        self.assertEqual("reusable", result_status(decision, "test_evidence"))
        self.assertEqual("revalidate", result_status(decision, "merge_readiness"))

    def test_wrong_task_identity_blocks_instead_of_reusing_evidence(self):
        evidence = evidence_set()
        evidence["tests"][0]["task_uid"] = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        decision = self.evaluate(evidence=evidence)

        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))
        self.assertTrue(decision.blockers)

    def test_review_from_a_different_source_head_is_blocked(self):
        evidence = evidence_set()
        evidence["reviews"][0]["source_head_oid"] = "f" * 40
        decision = self.evaluate(evidence=evidence)

        self.assertEqual("blocked", result_status(decision, "source_review"))
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))

    def test_wrong_pr_identity_and_duplicate_unit_evidence_block(self):
        target = target_snapshot()
        target["pr_number"] = 8
        decision = self.evaluate(target=target)
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))

        evidence = evidence_set()
        evidence["tests"].append(dict(evidence["tests"][0]))
        decision = self.evaluate(evidence=evidence)
        self.assertEqual("blocked", result_status(decision, "test_evidence"))

    def test_missing_run_attempt_or_artifact_provenance_blocks_test_reuse(self):
        for field, value in (("run_attempt", 0), ("check_run_id", None), ("artifact_id", None)):
            with self.subTest(field=field):
                evidence = evidence_set()
                evidence["tests"][0][field] = value
                decision = self.evaluate(evidence=evidence)
                self.assertEqual("blocked", result_status(decision, "test_evidence"))
                self.assertEqual("blocked", result_status(decision, "merge_readiness"))

    def test_missing_check_app_identity_blocks_test_reuse(self):
        missing_app = evidence_set()
        del missing_app["tests"][0]["check_app_id"]
        decision = self.evaluate(evidence=missing_app)
        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))

    def test_wrong_check_app_identity_blocks_test_reuse(self):
        wrong_app = evidence_set()
        wrong_app["tests"][0]["check_app_id"] = 41
        decision = self.evaluate(evidence=wrong_app)
        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))

    def test_missing_trusted_check_app_policy_blocks_reuse(self):
        decision = self.evaluate(policy={"enabled_capabilities": [CAPABILITY]})

        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))

    def test_failed_test_attempt_is_a_blocker_even_when_older_inputs_match(self):
        evidence = evidence_set()
        evidence["tests"][0]["status"] = "failed"
        decision = self.evaluate(evidence=evidence)

        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertEqual("blocked", result_status(decision, "merge_readiness"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
