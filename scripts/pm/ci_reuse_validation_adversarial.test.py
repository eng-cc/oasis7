#!/usr/bin/env python3
"""Adversarial checks for false authority in the validation contract API."""

import importlib.util
import dataclasses
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).parent
FIXTURE_SPEC = importlib.util.spec_from_file_location(
    "ci_reuse_validation_contract_fixtures", HERE / "ci_reuse_validation_contract.test.py",
)
fixtures = importlib.util.module_from_spec(FIXTURE_SPEC)
assert FIXTURE_SPEC.loader is not None
sys.modules[FIXTURE_SPEC.name] = fixtures
FIXTURE_SPEC.loader.exec_module(fixtures)
contract = fixtures.contract


class UntrustedAuthorityConstructionTests(unittest.TestCase):
    def test_caller_cannot_set_resolver_only_authority_fields(self):
        request = {
            "task_uid": "task_" + "a" * 32,
            "pr_number": contract.PR_NUMBER,
            "head_oid": "1" * 40,
            "integration_base_oid": "4" * 40,
            "source_scope_oid": "2" * 40,
            "projection_digest": "sha256:" + "3" * 64,
            "validation_units": ["required_gate_baseline"],
            "purpose": contract.PURPOSE,
            "request_digest": "sha256:" + "8" * 64,
        }
        # These fields used to let a caller claim the resolver and live-context
        # binding steps had already succeeded.
        with self.assertRaises(contract.ContractError):
            contract.ValidationAuthority(
                request=request,
                authorization={"unresolved": True},
                pin={"unresolved": True},
                request_comment_id=101,
                authorization_comment_id=100,
                pin_comment_id=102,
                request_body_digest="sha256:" + "9" * 64,
                authorization_body_digest="sha256:" + "7" * 64,
                pin_body_digest="sha256:" + "6" * 64,
                authorized_actor="self-approved",
                pin_actor="self-pinned",
                approval_permission="admin",
                pin_permission="admin",
                permission_snapshot_bound=True,
                validation_id=contract.validation_id(request["request_digest"]),
                planner_unit_obligations={
                    "required_gate_baseline": ("required_gate_baseline:000:baseline",)
                },
                context_bound=True,
                comment_timestamps=(),
            )

    def test_public_dataclass_replace_cannot_change_a_resolved_authority(self):
        *_, authority = fixtures.validation_fixture()
        with self.assertRaises(contract.ContractError):
            dataclasses.replace(authority, context_bound=False)

        unsealed = contract.ValidationAuthority.__new__(contract.ValidationAuthority)
        with self.assertRaises(contract.ContractError):
            contract.expected_run_title(unsealed)

    def test_provisional_authority_cannot_be_promoted_through_instance_dict(self):
        _, _, _, _, comments, permissions, bound_authority = fixtures.validation_fixture()
        authority = contract.resolve_records(comments, permissions)
        self.assertFalse(authority.context_bound)

        # Frozen dataclasses still expose a writable __dict__ unless slots are
        # enabled. This public reflection path must not let callers bypass the
        # context binder or substitute planner obligations.
        with self.assertRaises(TypeError):
            vars(authority)["context_bound"] = True
        self.assertFalse(authority.context_bound)

        run, check, payload = fixtures.payload_fixture(bound_authority)
        with self.assertRaises(contract.ContractError):
            contract.verify_payload(payload, authority, run, check)


if __name__ == "__main__":
    unittest.main()
