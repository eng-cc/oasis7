#!/usr/bin/env python3
"""Focused C3 executor-contract and request-journal contract tests."""
from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

import integration_executor_contract as contract


def effective_policy(**changes):
    value = {
        "enabled_capabilities": ["input-scope-reuse/v1"],
        "approved_executor_contract_digests": ["sha256:" + "e" * 64],
        "check_app_id": 42,
    }
    value.update(changes)
    return value


def request_identity(**changes):
    value = {
        "repository": "owner/repo",
        "task_uid": "task_" + "1" * 32,
        "pr_number": 12,
        "bootstrap_epoch": 1,
        "source_head_oid": "a" * 40,
        "publication_id": "pub-7",
        "source_projection_digest": "sha256:" + "b" * 64,
        "unit_ids": ["docs", "required-gate"],
        "input_fingerprints": {
            "docs": "sha256:" + "c" * 64,
            "required-gate": "sha256:" + "d" * 64,
        },
        "executor_contract_digest": "sha256:" + "e" * 64,
        "effective_policy_digest": contract.effective_policy_digest(effective_policy()),
        "purpose": "integration_revalidation",
        "applicability_mode": "input_scoped",
        "snapshot_target_oid": None,
    }
    value.update(changes)
    return value


class ExecutorContractTests(unittest.TestCase):
    def setUp(self):
        self.contents = {
            path: ("contract fixture: " + path).encode()
            for path in contract.EXECUTOR_CONTRACT_PATHS
        }
        self.executor = contract.executor_contract_from_contents(self.contents)

    def test_contract_is_canonical_and_binds_every_executor_file(self):
        reverse = dict(reversed(list(self.contents.items())))
        self.assertEqual(
            self.executor,
            contract.executor_contract_from_contents(reverse),
        )
        self.assertEqual(
            list(contract.EXECUTOR_CONTRACT_PATHS),
            [item["path"] for item in self.executor["files"]],
        )
        self.assertEqual(self.executor["digest"], contract.validate_executor_contract(self.executor))

    def test_missing_extra_and_changed_contract_content_fail_closed(self):
        for contents in (
            {key: value for key, value in self.contents.items() if key != contract.EXECUTOR_CONTRACT_PATHS[0]},
            {**self.contents, "candidate-extra.py": b"untrusted"},
        ):
            with self.subTest(keys=sorted(contents)):
                with self.assertRaisesRegex(ValueError, "closure is incomplete"):
                    contract.executor_contract_from_contents(contents)
        changed = copy.deepcopy(self.executor)
        changed["files"][0]["sha256"] = "sha256:" + "f" * 64
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            contract.validate_executor_contract(changed)

    def test_executor_requires_separately_approved_contract_digest(self):
        self.assertEqual(
            self.executor["digest"],
            contract.require_approved_executor_contract(
                self.executor, [self.executor["digest"]],
            ),
        )
        with self.assertRaisesRegex(ValueError, "EXECUTOR_CONTRACT_CHANGED"):
            contract.require_approved_executor_contract(
                self.executor, ["sha256:" + "0" * 64],
            )
        with self.assertRaisesRegex(ValueError, "policy is missing"):
            contract.require_approved_executor_contract(self.executor, [])


class ValidationRequestTests(unittest.TestCase):
    def test_request_key_binds_inputs_but_not_first_frozen_base(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            first, created = contract.reserve_validation_request(
                directory, key, identity, "1" * 40,
            )
            retry, retried = contract.reserve_validation_request(
                directory, key, identity, "2" * 40,
            )
        self.assertTrue(created)
        self.assertFalse(retried)
        self.assertEqual("1" * 40, first["integration_base_oid"])
        self.assertEqual("1" * 40, retry["integration_base_oid"])

        changed = request_identity(input_fingerprints={
            "docs": "sha256:" + "9" * 64,
            "required-gate": "sha256:" + "d" * 64,
        })
        self.assertNotEqual(key, contract.validation_request_key(changed))
        changed_executor = request_identity(executor_contract_digest="sha256:" + "8" * 64)
        self.assertNotEqual(key, contract.validation_request_key(changed_executor))
        changed_policy = request_identity(
            effective_policy_digest=contract.effective_policy_digest(
                effective_policy(approved_executor_contract_digests=["sha256:" + "8" * 64]),
            ),
        )
        self.assertNotEqual(key, contract.validation_request_key(changed_policy))

    def test_intent_order_is_durable_monotonic_and_stable_for_same_key_retry(self):
        first_identity = request_identity()
        second_identity = request_identity(
            source_projection_digest="sha256:" + "9" * 64,
        )
        first_key = contract.validation_request_key(first_identity)
        second_key = contract.validation_request_key(second_identity)
        with tempfile.TemporaryDirectory() as directory:
            first, created = contract.reserve_validation_request(
                directory, first_key, first_identity, "1" * 40,
            )
            retry, retried = contract.reserve_validation_request(
                directory, first_key, first_identity, "2" * 40,
            )
            second, second_created = contract.reserve_validation_request(
                directory, second_key, second_identity, "3" * 40,
            )
            self.assertEqual(3, contract.validate_validation_intent_order_state(directory))

        self.assertTrue(created)
        self.assertFalse(retried)
        self.assertTrue(second_created)
        self.assertEqual(1, first["intent_order"])
        self.assertEqual(first["intent_order"], retry["intent_order"])
        self.assertEqual(2, second["intent_order"])

    def test_intent_order_counter_corruption_or_disappearance_fails_closed(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            contract.reserve_validation_request(directory, key, identity, "1" * 40)
            state_path = Path(directory) / ".validation-intent-order"
            state_path.write_text(json.dumps({
                "schema": contract.VALIDATION_INTENT_ORDER_SCHEMA,
                "next_order": 2,
                "orders": {},
            }), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "sequence is inconsistent"):
                contract.validate_validation_intent_order_state(directory)
            state_path.unlink()
            with self.assertRaisesRegex(ValueError, "state is missing"):
                contract.validate_validation_intent_order_state(directory)

    def test_deleting_a_modern_journal_row_fails_closed(self):
        first_identity = request_identity()
        second_identity = request_identity(
            source_projection_digest="sha256:" + "9" * 64,
        )
        first_key = contract.validation_request_key(first_identity)
        second_key = contract.validation_request_key(second_identity)
        with tempfile.TemporaryDirectory() as directory:
            contract.reserve_validation_request(directory, first_key, first_identity, "1" * 40)
            contract.reserve_validation_request(directory, second_key, second_identity, "2" * 40)
            contract._request_path(Path(directory), second_key).unlink()
            with self.assertRaisesRegex(ValueError, "sequence is inconsistent"):
                contract.validate_validation_intent_order_state(directory)

    def test_legacy_journal_remains_readable_but_cannot_start_unordered_dispatch(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            record, _ = contract.reserve_validation_request(
                directory, key, identity, "1" * 40,
            )
            path = Path(directory) / (key.removeprefix("sha256:") + ".json")
            record.pop("intent_order")
            record.pop("intent_order_schema")
            path.write_text(json.dumps(record), encoding="utf-8")
            (Path(directory) / ".validation-intent-order").unlink()
            self.assertNotIn("intent_order", contract._read_request_record(path, key))
            retry, created = contract.reserve_validation_request(
                directory, key, identity, "1" * 40,
            )
            self.assertFalse(created)
            self.assertNotIn("intent_order", retry)
            with self.assertRaisesRegex(ValueError, "without immutable intent order"):
                contract.mark_validation_dispatch_started(directory, key)

    def test_partial_intent_order_marker_is_not_misread_as_legacy(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            contract.reserve_validation_request(directory, key, identity, "1" * 40)
            path = Path(directory) / (key.removeprefix("sha256:") + ".json")
            record = json.loads(path.read_text(encoding="utf-8"))
            del record["intent_order"]
            path.write_text(json.dumps(record), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "journal fields"):
                contract._read_request_record(path, key)

    def test_epoch_is_a_canonical_positive_integer(self):
        for value in (True, False, 0, -1, "1", "epoch-1"):
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "bootstrap epoch"):
                contract.validation_request_identity(request_identity(bootstrap_epoch=value))
        self.assertEqual(1, contract.validation_request_identity(request_identity())["bootstrap_epoch"])

    def test_request_envelope_binds_key_identity_and_frozen_base(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        envelope = {
            "schema": contract.VALIDATION_REQUEST_SCHEMA,
            "request_key": key,
            "identity": identity,
            "integration_base_oid": "1" * 40,
        }
        self.assertEqual(envelope, contract.validation_request_envelope(envelope))
        for changed in (
            {**envelope, "request_key": "sha256:" + "0" * 64},
            {**envelope, "integration_base_oid": True},
            {**envelope, "identity": request_identity(bootstrap_epoch=2)},
            {**envelope, "unexpected": "field"},
        ):
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                contract.validation_request_envelope(changed)

    def test_snapshot_exact_key_binds_its_target(self):
        value = request_identity(
            applicability_mode="snapshot_exact", snapshot_target_oid="f" * 40,
        )
        key = contract.validation_request_key(value)
        changed = {**value, "snapshot_target_oid": "e" * 40}
        self.assertNotEqual(key, contract.validation_request_key(changed))

    def test_incomplete_or_ambiguous_request_closure_is_rejected(self):
        cases = (
            request_identity(unit_ids=["docs", "docs"]),
            request_identity(input_fingerprints={"docs": "sha256:" + "c" * 64}),
            request_identity(applicability_mode="snapshot_exact"),
            request_identity(applicability_mode="input_scoped", snapshot_target_oid="f" * 40),
        )
        for value in cases:
            with self.subTest(value=value), self.assertRaises(ValueError):
                contract.validation_request_key(value)

    def test_lost_response_recovery_is_bounded_and_binds_one_run(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            record, _ = contract.reserve_validation_request(directory, key, identity, "1" * 40)
            self.assertEqual("prepared", record["status"])
            record = contract.mark_validation_dispatch_started(directory, key)
            self.assertEqual(1, record["dispatch_attempts"])
            self.assertEqual("dispatch_uncertain", record["status"])
            observed = contract.mark_validation_request_observed(directory, key, 9001, 1)
            self.assertEqual("observed", observed["status"])
            self.assertEqual(9001, observed["run_id"])
            with self.assertRaisesRegex(ValueError, "different run"):
                contract.mark_validation_request_observed(directory, key, 9002, 1)

    def test_request_journal_rejects_unrecognized_or_noncanonical_identity_fields(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            contract.reserve_validation_request(directory, key, identity, "1" * 40)
            path = Path(directory) / (key.removeprefix("sha256:") + ".json")
            record = json.loads(path.read_text(encoding="utf-8"))
            record["unexpected"] = "untrusted extension"
            path.write_text(json.dumps(record), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "journal fields"):
                contract.reserve_validation_request(directory, key, identity, "1" * 40)

    def test_request_dispatch_retry_limit_is_finite(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        with tempfile.TemporaryDirectory() as directory:
            contract.reserve_validation_request(directory, key, identity, "1" * 40)
            record = contract.mark_validation_dispatch_started(directory, key)
            self.assertEqual(1, record["dispatch_attempts"])
            with self.assertRaisesRegex(ValueError, "VALIDATION_REQUEST_RETRY_LIMIT"):
                contract.mark_validation_dispatch_started(directory, key)

    def test_delayed_empty_readback_keeps_uncertain_request_pending_without_redispatch(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        sends = []

        with tempfile.TemporaryDirectory() as directory:
            first, disposition = contract.ensure_validation_request(
                directory, key, identity, "1" * 40,
                readback=lambda *_args: None,
                dispatch=lambda record: sends.append(record["request_key"]),
            )
            retry, retry_disposition = contract.ensure_validation_request(
                directory, key, identity, "2" * 40,
                readback=lambda *_args: None,
                dispatch=lambda _record: self.fail("uncertain key must not dispatch twice"),
            )

        self.assertEqual("pending", disposition)
        self.assertEqual("pending", retry_disposition)
        self.assertEqual(1, first["dispatch_attempts"])
        self.assertEqual("1" * 40, retry["integration_base_oid"])
        self.assertEqual([key], sends)

    def test_dispatch_response_loss_recovers_by_key_without_duplicate_send(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        remote = []
        sends = []

        def readback(request_key, base):
            return next((item for item in remote if item["request_key"] == request_key
                         and item["integration_base_oid"] == base), None)

        def lost_response(record):
            sends.append(record["request_key"])
            remote.append({
                "request_key": record["request_key"],
                "integration_base_oid": record["integration_base_oid"],
                "run_id": 7001,
                "run_attempt": 1,
            })
            raise OSError("dispatch response lost")

        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(OSError, "response lost"):
                contract.ensure_validation_request(
                    directory, key, identity, "1" * 40,
                    readback=readback, dispatch=lost_response,
                )
            record, disposition = contract.ensure_validation_request(
                directory, key, identity, "2" * 40,
                readback=readback,
                dispatch=lambda _record: self.fail("same-key retry must read back first"),
            )
        self.assertEqual("reused", disposition)
        self.assertEqual("1" * 40, record["integration_base_oid"])
        self.assertEqual(7001, record["run_id"])
        self.assertEqual([key], sends)

    def test_uncertain_readback_never_dispatches(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        sends = []
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(OSError, "coverage unavailable"):
                contract.ensure_validation_request(
                    directory, key, identity, "1" * 40,
                    readback=lambda *_args: (_ for _ in ()).throw(OSError("coverage unavailable")),
                    dispatch=lambda record: sends.append(record),
                )
        self.assertEqual([], sends)

    def test_missing_previously_observed_request_never_falls_back_to_redispatch(self):
        identity = request_identity()
        key = contract.validation_request_key(identity)
        found = {
            "request_key": key,
            "integration_base_oid": "1" * 40,
            "run_id": 7001,
            "run_attempt": 1,
        }
        sends = []
        with tempfile.TemporaryDirectory() as directory:
            contract.reserve_validation_request(directory, key, identity, "1" * 40)
            contract.mark_validation_dispatch_started(directory, key)
            contract.mark_validation_request_observed(directory, key, 7001, 1)
            with self.assertRaisesRegex(ValueError, "previously observed"):
                contract.ensure_validation_request(
                    directory, key, identity, "1" * 40,
                    readback=lambda *_args: None,
                    dispatch=lambda record: sends.append(record),
                )
        self.assertEqual([], sends)


if __name__ == "__main__":
    unittest.main()
