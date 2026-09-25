#!/usr/bin/env python3
"""Focused contract tests for the proposed CI applicability reader/evaluator."""

import unittest

import ci_evidence_applicability as applicability
import ci_input_scope as input_scope
import ci_ready_receipt_identity as receipt_identity
import projection_publication_contract as publication


UID = "task_12345678901234567890123456789012"
REPOSITORY = "eng-cc/oasis7"
HEAD = "a" * 40
TARGET = "b" * 40
SOURCE_SCOPE = "9" * 40
DIGEST = f"sha256:{'c' * 64}"
INPUT_DIGEST = f"sha256:{'d' * 64}"
CORPUS_UNIT = "product-corpus:membership"
CAPABILITY = "input-scope-reuse/v1"
PLAN_SCHEMA = "oasis7-required-plan-v2"


def product_corpus():
    return input_scope.product_corpus_descriptor(
        [CORPUS_UNIT],
        [{"obligation_id": CORPUS_UNIT, "unit_id": CORPUS_UNIT}],
    )


def planner_unit_specs():
    def unit(unit_id, obligation):
        return {
            "unit_id": unit_id,
            "unit_contract": {"kind": "fixture-unit/v1", "id": unit_id},
            "obligation_set": [obligation],
            "command_checker_paths": ["scripts/check-ci.py"],
            "input_paths": ["src/" + unit_id + ".rs"],
            "member_roots": [],
            "dependency_edges": [],
            "applicable_policy": {"fixture": "policy/v1"},
            "environment_contract": {"fixture": "python/v1"},
        }
    return [
        unit(CORPUS_UNIT, CORPUS_UNIT),
        unit("unit-a", "obligation:unit-a"),
    ]


def planner_inventory_issuer(target_oid, target_tree_oid):
    specs = planner_unit_specs()
    corpus = product_corpus()
    return {
        "schema": input_scope.TRUSTED_PLANNER_INVENTORY_SCHEMA,
        "authority": {
            "schema": input_scope.PLANNER_INVENTORY_AUTHORITY_SCHEMA,
            "repository": REPOSITORY,
            "workflow_ref": f"{REPOSITORY}/.github/workflows/rust.yml@refs/heads/main",
            "planner_authority_oid": "e" * 40,
            "planner_config_sha256": f"sha256:{'f' * 64}",
        },
        "producer": {
            "run_id": 10,
            "run_attempt": 1,
            "check_app_id": 42,
            "check_run_id": 20,
        },
        "target_oid": target_oid,
        "target_tree_oid": target_tree_oid,
        "unit_ids": sorted(spec["unit_id"] for spec in specs),
        "inventory_digest": input_scope.planner_inventory_digest(
            specs, corpus, target_oid, target_tree_oid,
        ),
    }


def trusted_inventory_readback(issuer, *, artifact_id=30):
    return {**issuer, "producer": {**issuer["producer"], "artifact_id": artifact_id}}


def source_plan():
    return {
        "schema": PLAN_SCHEMA,
        "required_capabilities": [CAPABILITY],
        "repository": REPOSITORY,
        "task_uid": UID,
        "pr_number": 7,
        "source_head_oid": HEAD,
        "source_scope_oid": SOURCE_SCOPE,
        "review_applicability_digest": DIGEST,
        "required_test_units": [CORPUS_UNIT, "unit-a"],
        "required_review_roles": ["runtime_engineer"],
        "planner_inventory_issuer": planner_inventory_issuer(HEAD, HEAD),
    }


def target_snapshot(*, input_digest=INPUT_DIGEST, review_digest=DIGEST, closure="complete"):
    if closure == "complete":
        scope = {
            "schema": input_scope.INPUT_SCOPE_SCHEMA,
            "target_oid": TARGET,
            "target_tree_oid": TARGET,
            "planner_inventory_issuer": planner_inventory_issuer(TARGET, TARGET),
            "closure_status": {"status": "complete", "reason": None},
            "required_test_units": [CORPUS_UNIT, "unit-a"],
            "input_fingerprints": {"unit-a": input_digest, CORPUS_UNIT: INPUT_DIGEST},
            "dependency_edges": [],
            "fallback_complete": True,
            "fallback_contract": None,
            "product_corpus": product_corpus(),
        }
    else:
        scope = {
            "schema": input_scope.INPUT_SCOPE_SCHEMA,
            "target_oid": TARGET,
            "target_tree_oid": TARGET,
            "planner_inventory_issuer": planner_inventory_issuer(TARGET, TARGET),
            "closure_status": {"status": "unknown", "reason": "fixture closure unavailable"},
            "required_test_units": [CORPUS_UNIT, "unit-a"],
            "input_fingerprints": {},
            "dependency_edges": [],
            "fallback_complete": True,
            "fallback_contract": input_scope.planner_fallback_contract(
                [CORPUS_UNIT, "unit-a"], TARGET, TARGET,
            ),
            "product_corpus": product_corpus(),
        }
    return {
        "repository": REPOSITORY,
        "task_uid": UID,
        "pr_number": 7,
        "source_head_oid": HEAD,
        "source_scope_oid": SOURCE_SCOPE,
        "target_oid": TARGET,
        "prior_assessed_target_oid": None,
        "input_scope_commit_oid": TARGET,
        "input_scope_tree_oid": TARGET,
        "review_applicability_digest": review_digest,
        "required_test_units": [CORPUS_UNIT, "unit-a"],
        "required_review_roles": ["runtime_engineer"],
        "input_scope": scope,
    }


def evidence_set(*, test_input_digest=INPUT_DIGEST, review_digest=DIGEST):
    return {
        "reviews": [{
            "role_id": "runtime_engineer",
            "status": "passed",
            "evidence_locator": {"kind": "task-issue-comment", "id": "5825000000"},
            "repository": REPOSITORY,
            "task_uid": UID,
            "pr_number": 7,
            "source_head_oid": HEAD,
            "source_scope_oid": SOURCE_SCOPE,
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
            "source_scope_oid": SOURCE_SCOPE,
            "run_id": 10,
            "run_attempt": 1,
            "check_app_id": 42,
            "check_run_id": 20,
            "artifact_id": 30,
            "inventory_digest": planner_inventory_issuer(HEAD, HEAD)["inventory_digest"],
            "effective_policy_identity": effective_policy_identity(),
        }, {
            "unit_id": CORPUS_UNIT,
            "obligation_ids": [CORPUS_UNIT],
            "status": "passed",
            "input_digest": INPUT_DIGEST,
            "repository": REPOSITORY,
            "task_uid": UID,
            "pr_number": 7,
            "source_head_oid": HEAD,
            "source_scope_oid": SOURCE_SCOPE,
            "run_id": 10,
            "run_attempt": 1,
            "check_app_id": 42,
            "check_run_id": 20,
            "artifact_id": 31,
            "inventory_digest": planner_inventory_issuer(HEAD, HEAD)["inventory_digest"],
            "effective_policy_identity": effective_policy_identity(),
        }],
    }


def enabled_policy():
    return {
        "enabled_capabilities": [CAPABILITY],
        "approved_executor_contract_digests": [],
        "check_app_id": 42,
    }


def effective_policy_identity(policy=None):
    from integration_executor_contract import (
        EFFECTIVE_POLICY_IDENTITY_SCHEMA,
        effective_policy_digest,
    )
    return {
        "schema": EFFECTIVE_POLICY_IDENTITY_SCHEMA,
        "digest": effective_policy_digest(enabled_policy() if policy is None else policy),
    }


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
        plan = source_plan() if plan is None else plan
        target = target_snapshot() if target is None else target
        return applicability.evaluate_evidence_applicability(
            plan,
            evidence_set() if evidence is None else evidence,
            target,
            enabled_policy() if policy is None else policy,
            trusted_source_inventory=trusted_inventory_readback(
                plan["planner_inventory_issuer"], artifact_id=30,
            ),
            trusted_target_inventory=trusted_inventory_readback(
                target.get("input_scope", target_snapshot()["input_scope"])["planner_inventory_issuer"],
                artifact_id=31,
            ),
        )

    def test_reuse_is_disabled_when_policy_omits_capability(self):
        policy = enabled_policy()
        policy["enabled_capabilities"] = []
        decision = self.evaluate(policy=policy)

        self.assertEqual("disabled", result_status(decision, "source_review"))
        self.assertEqual("disabled", result_status(decision, "test_evidence"))
        self.assertEqual("disabled", result_status(decision, "merge_readiness"))
        self.assertEqual(effective_policy_identity(policy), decision.effective_policy_identity)

    def test_unknown_policy_capability_blocks(self):
        decision = self.evaluate(policy={"enabled_capabilities": ["future-capability/v1"]})

        self.assertEqual("blocked", result_status(decision, "merge_readiness"))
        self.assertTrue(decision.blockers)

    def test_exact_identity_and_provenance_allow_both_evidence_dimensions(self):
        decision = self.evaluate()

        self.assertEqual("reusable", result_status(decision, "source_review"))
        self.assertEqual("reusable", result_status(decision, "test_evidence"))
        self.assertEqual("reusable", result_status(decision, "merge_readiness"))
        self.assertEqual((CORPUS_UNIT, "unit-a"), tuple(decision.reused_units))
        self.assertEqual({
            "source_head_oid": HEAD,
            "source_scope_oid": SOURCE_SCOPE,
            "assessed_target_oid": TARGET,
            "prior_assessed_target_oid": None,
        }, decision.identity)
        self.assertEqual("sha256:", decision.effective_policy_identity["digest"][:7])
        self.assertEqual({"review", "test"}, {row["kind"] for row in decision.item_decisions})
        self.assertTrue(all(row["reason"] for row in decision.item_decisions))
        self.assertTrue(decision.evidence_locators)

    def test_changed_test_input_revalidates_tests_without_invalidating_review(self):
        decision = self.evaluate(
            target=target_snapshot(input_digest=f"sha256:{'e' * 64}"),
        )

        self.assertEqual("reusable", result_status(decision, "source_review"))
        self.assertEqual("revalidate", result_status(decision, "test_evidence"))
        self.assertEqual("revalidate", result_status(decision, "merge_readiness"))
        self.assertEqual(("unit-a",), tuple(decision.required_test_units))
        row = next(item for item in decision.item_decisions if item["id"] == "unit-a")
        self.assertEqual("TEST_INPUT_FINGERPRINT_CHANGED", row["reason"])
        self.assertEqual(10, row["evidence_locator"]["id"]["run_id"])

    def test_unknown_input_closure_widens_all_units_and_prevents_reuse(self):
        decision = self.evaluate(target=target_snapshot(closure="unknown"))

        self.assertEqual("reusable", result_status(decision, "source_review"))
        self.assertEqual("revalidate", result_status(decision, "test_evidence"))
        self.assertEqual("revalidate", result_status(decision, "merge_readiness"))
        self.assertEqual((CORPUS_UNIT, "unit-a"), tuple(decision.required_test_units))
        self.assertEqual((), tuple(decision.reused_units))

    def test_input_scope_is_required_before_capability_evidence_can_be_consumed(self):
        target = target_snapshot()
        del target["input_scope"]
        decision = self.evaluate(target=target)

        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertIn("APPLICABILITY_INPUT_INVALID", decision.blockers)

    def test_assessed_target_is_distinct_from_input_scope_commit(self):
        target = target_snapshot()
        target["target_oid"] = "e" * 40
        decision = self.evaluate(target=target)

        self.assertEqual("reusable", result_status(decision, "source_review"))
        self.assertEqual("reusable", result_status(decision, "test_evidence"))
        self.assertEqual("reusable", result_status(decision, "merge_readiness"))
        self.assertEqual("e" * 40, decision.identity["assessed_target_oid"])

    def test_product_result_must_cover_the_full_obligation_set(self):
        evidence = evidence_set()
        evidence["tests"][1]["obligation_ids"] = []
        decision = self.evaluate(evidence=evidence)

        self.assertEqual("blocked", result_status(decision, "test_evidence"))
        self.assertTrue(any("PRODUCT_CORPUS_OBLIGATION_MISMATCH" in item for item in decision.blockers))

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

    def test_same_head_with_different_source_scope_is_blocked(self):
        evidence = evidence_set()
        evidence["reviews"][0]["source_scope_oid"] = "8" * 40
        decision = self.evaluate(evidence=evidence)

        self.assertEqual("blocked", result_status(decision, "source_review"))
        self.assertIn("REVIEW_IDENTITY_MISMATCH", decision.blockers)

    def test_missing_trusted_planner_inventory_binding_blocks(self):
        decision = applicability.evaluate_evidence_applicability(
            source_plan(), evidence_set(), target_snapshot(), enabled_policy(),
        )

        self.assertEqual("blocked", result_status(decision, "merge_readiness"))
        self.assertIn("APPLICABILITY_INPUT_INVALID", decision.blockers)

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
