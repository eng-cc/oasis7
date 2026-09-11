#!/usr/bin/env python3
"""RED contract for the W2 coordinating-record traceability checker.

The implementation is deliberately absent during RED.  The sentinel returned
by ``_load_api`` turns that absence into a behavioral result so assertions
identify the missing contract rather than failing at import time.
"""

from copy import deepcopy
from contextlib import redirect_stdout
import hashlib
import io
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch


HERE = Path(__file__).resolve().parent
SOURCE_OID = "06d6754b5311bceb2b904d7a17e2991b217f501b"
TASK_UID = "task_" + "a" * 32
LEAF_UID = "task_" + "b" * 32
SECOND_LEAF_UID = "task_" + "c" * 32
CHANGE_ID = "change-3671-traceability"
RECORD_COMMENT_ID = 5636938574
EQUIVALENCE_APPROVAL_COMMENT_ID = 5636906114
REPOSITORY = "eng-cc/oasis7"
CANDIDATE_FIELDS = (
    "change_id",
    "source_head_oid",
    "integration_base_oid",
    "tested_tree_oid",
    "configuration_digest",
    "entry",
    "environment",
    "evidence_window",
    "effective_policy_identity",
    "effective_helper_identity",
    "effective_workflow_identity",
    "consumed_contracts",
)


def _canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def _record_digest(record):
    """Digest the record with its self-referential digest slot blanked."""
    value = deepcopy(record)
    value["coordination_ref"]["record_digest"] = ""
    return "sha256:" + hashlib.sha256(_canonical(value).encode()).hexdigest()


def _load_api():
    path = HERE / "loop_traceability.py"
    if path.exists():
        spec = importlib.util.spec_from_file_location("loop_traceability", path)
        if spec is None or spec.loader is None:
            raise AssertionError("loop_traceability.py has no import loader")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    class MissingTraceabilityImplementation:
        """Keep RED failures behavioral while the GREEN helper is absent."""

        def __getattr__(self, name):
            def missing(*args, **kwargs):
                return {
                    "status": "blocked",
                    "blockers": [
                        f"W2 traceability implementation missing: loop_traceability.{name}"
                    ],
                }

            return missing

    return MissingTraceabilityImplementation()


def _contract_ref(path="doc/engineering/workflow/source-of-truth.md", fragment="traceability-record-contract", clause_id="QW2-1"):
    return {
        "repository": REPOSITORY,
        "path": path,
        "fragment": fragment,
        "contract_id": "engineering-workflow",
        "revision": "v1.15.2",
        "contract_digest": "sha256:" + "1" * 64,
        "publication_ref": {
            "repository": REPOSITORY,
            "issue_number": 3671,
            "comment_id": 5636639918,
        },
        "clause_id": clause_id,
    }


def _record():
    record = {
        "schema": "oasis7.loop-change/v1",
        "marker": "oasis7-loop-change-record",
        "task_uid": TASK_UID,
        "change_id": CHANGE_ID,
        "coordination_ref": {
            "repository": REPOSITORY,
            "issue_number": 3671,
            "comment_id": RECORD_COMMENT_ID,
            "record_digest": "",
            "source_commit": SOURCE_OID,
        },
        "required_obligations": [
            {
                "obligation_id": "QW2-1",
                "mapping_slot": "slot-contract",
                "required": True,
                "acceptance_refs": [_contract_ref(clause_id="QW2-1")],
                "owner_loop": "code",
                "owner_role": "repository_health_engineer",
            },
            {
                "obligation_id": "QW2-3",
                "mapping_slot": "slot-handoff",
                "required": True,
                "acceptance_refs": [_contract_ref(fragment="manual-three-loop-transition", clause_id="QW2-3")],
                "owner_loop": "system",
                "owner_role": "producer_system_designer",
            },
        ],
        "mapping_slots": [
            {"slot_id": "slot-contract", "owner_loop": "code", "owner_role": "repository_health_engineer"},
            {"slot_id": "slot-handoff", "owner_loop": "system", "owner_role": "producer_system_designer"},
        ],
        "candidate_selection": {
            "comparable_fields": [
                "change_id",
                "source_head_oid",
                "integration_base_oid",
                "tested_tree_oid",
                "configuration_digest",
                "entry",
                "environment",
                "evidence_window",
                "effective_policy_identity",
                "effective_helper_identity",
                "effective_workflow_identity",
                "consumed_contracts",
            ],
            "required_integration_base_oid": "d" * 40,
            "required_tested_tree_oid": "e" * 40,
        },
        "feedback": [],
    }
    record["coordination_ref"]["record_digest"] = _record_digest(record)
    return record


def _binding(**updates):
    binding = {
        "schema": "oasis7.loop-task/v1",
        "task_uid": LEAF_UID,
        "change_id": CHANGE_ID,
        "loop": "code",
        "owner_role": "repository_health_engineer",
        "bootstrap_epoch": 1,
        "manual_request_ref": "issuecomment-5636639918",
        "request_key": "change-3671-traceability/slot-contract",
        "write_scope": ["scripts/pm/**"],
        "out_of_scope": [],
        "acceptance_refs": [_contract_ref(clause_id="QW2-1")],
        "dependencies": [],
        "target_delivery": "pilot",
        "coordination_ref": deepcopy(_record()["coordination_ref"]),
    }
    binding.update(updates)
    return binding


def _candidate(record, source_head="f" * 40):
    return {
        "change_id": record["change_id"],
        "source_head_oid": source_head,
        "integration_base_oid": record["candidate_selection"]["required_integration_base_oid"],
        "tested_tree_oid": record["candidate_selection"]["required_tested_tree_oid"],
        "configuration_digest": "sha256:" + "2" * 64,
        "entry": "scripts/pm/loop.py bind",
        "environment": "local",
        "evidence_window": {"started_at": "2026-09-11T00:00:00Z", "ended_at": "2026-09-11T00:01:00Z"},
        "effective_policy_identity": {"path": "scripts/pm/loop_policy.py", "commit": SOURCE_OID},
        "effective_helper_identity": {"path": "scripts/pm/loop_traceability.py", "commit": SOURCE_OID, "digest": "sha256:" + "4" * 64},
        "effective_workflow_identity": {"path": "doc/engineering/workflow/source-of-truth.md", "commit": SOURCE_OID, "digest": "sha256:" + "5" * 64},
        "consumed_contracts": [{
            "repository": REPOSITORY,
            "contract_id": "engineering-workflow",
            "revision": "v1.15.2",
            "digest": "sha256:" + "1" * 64,
            "publication_ref": {"repository": REPOSITORY, "issue_number": 3671, "comment_id": 5636906114},
        }],
    }


def _matrix_row(obligation_id, slot_id, uid, candidate, evidence_digest):
    return {
        "obligation_id": obligation_id,
        "mapping_slot": slot_id,
        "leaf_task_uid": uid,
        "leaf_evidence_locator": f"issuecomment-{uid}",
        "leaf_evidence_digest": evidence_digest,
        "source_head_oid": candidate["source_head_oid"],
        "integration_base_oid": candidate["integration_base_oid"],
        "tested_tree_oid": candidate["tested_tree_oid"],
        "configuration_digest": candidate["configuration_digest"],
        "entry": candidate["entry"],
        "environment": candidate["environment"],
        "evidence_window": candidate["evidence_window"],
        "effective_policy_identity": candidate["effective_policy_identity"],
        "effective_helper_identity": candidate["effective_helper_identity"],
        "effective_workflow_identity": candidate["effective_workflow_identity"],
        "consumed_contracts": candidate["consumed_contracts"],
    }


def _candidate_projection(candidate):
    return {key: deepcopy(candidate[key]) for key in CANDIDATE_FIELDS}


def _evidence_payload(uid, candidate):
    return {"task_uid": uid, "status": "passed", "candidate": _candidate_projection(candidate)}


def _evidence_digest(payload):
    return "sha256:" + hashlib.sha256(_canonical(payload).encode()).hexdigest()


def _authority_ref(comment_id=5636906114):
    return {"repository": REPOSITORY, "issue_number": 3671, "comment_id": comment_id}


def _equivalence_approval(record):
    candidate = _candidate(record, source_head="0" * 40)
    payload = {
        "marker": "oasis7-equivalence-approval",
        "schema": "oasis7.loop-equivalence-approval/v1",
        "task_uid": record["task_uid"],
        "change_id": record["change_id"],
        "approval": "approved",
        "approver_role": "producer_system_designer",
        "source_leaf": {"task_uid": SECOND_LEAF_UID, "source_head_oid": "0" * 40},
        "aggregate_candidate": {"change_id": record["change_id"], "tested_tree_oid": candidate["tested_tree_oid"]},
        "allowed_to_differ": ["source_head_oid"],
        "exact_fields": [
            "change_id",
            "integration_base_oid",
            "tested_tree_oid",
            "configuration_digest",
            "entry",
            "environment",
            "evidence_window",
        ],
        "supporting_evidence_digest": _evidence_digest(_evidence_payload(SECOND_LEAF_UID, candidate)),
    }
    return payload


def _rewrite_comment_body(payload, mutate):
    body = json.loads(payload["comment"]["body"])
    mutate(body)
    payload["comment"]["body"] = _canonical(body)


class FixtureReaders:
    def __init__(self, record):
        self.record = record
        self.authority_calls = []
        self.authority_results = []
        self.contract_calls = []
        self.comment = self._comment(record)
        self.equivalence_comment = {
            "id": EQUIVALENCE_APPROVAL_COMMENT_ID,
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/3671",
            "body": _canonical(_equivalence_approval(record)),
            "user": {"login": "producer-system-designer"},
            "created_at": "2026-09-11T00:00:00Z",
        }
        self.comments = {
            RECORD_COMMENT_ID: self.comment,
            EQUIVALENCE_APPROVAL_COMMENT_ID: self.equivalence_comment,
        }

    @staticmethod
    def _comment(record):
        body = _canonical({
            "marker": record["marker"],
            "schema": record["schema"],
            "task_uid": record["task_uid"],
            "change_id": record["change_id"],
            "record_digest": record["coordination_ref"]["record_digest"],
            "record": record,
        })
        return {
            "id": RECORD_COMMENT_ID,
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/3671",
            "body": body,
            "user": {"login": "coordinator"},
            "created_at": "2026-09-11T00:00:00Z",
        }

    def authority(self, *args, **kwargs):
        self.authority_calls.append((args, kwargs))
        reference = args[0] if args else kwargs.get("coordination_ref") or kwargs.get("authority_ref")
        comment_id = reference.get("comment_id") if isinstance(reference, dict) else RECORD_COMMENT_ID
        comment = self.comments.get(comment_id)
        if comment is None:
            return {
                "status": "blocked",
                "blockers": [f"unknown authority comment {comment_id}"],
                "reader_kind": "fixture_authority",
            }
        result = {
            "reader_kind": "fixture_authority",
            "repository": REPOSITORY,
            "issue": {
                "number": 3671,
                "html_url": f"https://github.com/{REPOSITORY}/issues/3671",
                "body": f"<!-- oasis7-pm-task -->\ntask_uid: {TASK_UID}\n",
            },
            "comment": comment,
        }
        self.authority_results.append(result)
        return result

    def contract(self, reference, *args, **kwargs):
        self.contract_calls.append((reference, args, kwargs))
        if reference.get("fragment") == "missing-fragment":
            return {
                "status": "blocked",
                "blockers": [
                    "unapproved consumed contract clause: "
                    + reference["path"]
                    + "#"
                    + reference["fragment"]
                ],
            }
        return {"status": "passed", "repository": REPOSITORY, "path": reference["path"], "fragment": reference["fragment"], "digest": reference["contract_digest"]}


class TraceabilityPreflight(Exception):
    pass


class _MissingLoopBoundary:
    def pre_mutation_admission(self, *args, **kwargs):
        return {"status": "blocked", "blockers": ["W2 pre-mutation boundary API missing: loop.pre_mutation_admission"]}


def _load_loop_boundary():
    path = HERE / "loop.py"
    spec = importlib.util.spec_from_file_location("loop_boundary", path)
    if spec is None or spec.loader is None:
        raise AssertionError("loop.py has no import loader")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module if hasattr(module, "pre_mutation_admission") else _MissingLoopBoundary()


def _load_loop_dispatch():
    """Load the real loop facade so tests exercise its command branches."""
    path = HERE / "loop.py"
    spec = importlib.util.spec_from_file_location("loop_dispatch", path)
    if spec is None or spec.loader is None:
        raise AssertionError("loop.py has no import loader")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CloseoutFixture:
    """Run the actual closeout entrypoint with only remote mutation stubs."""

    def __init__(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.script_dir = self.root / "scripts" / "pm"
        self.script_dir.mkdir(parents=True)
        shutil.copy2(HERE / "task-closeout.sh", self.script_dir / "task-closeout.sh")
        (self.script_dir / "task-closeout.sh").chmod(0o755)
        self.marker = self.root / "downstream-mutation.log"
        self._write_remote_stubs()
        mapping = self.root / ".pm" / "github-project-sync" / "tasks.json"
        mapping.parent.mkdir(parents=True)
        mapping.write_text(json.dumps({
            "version": 1,
            "tasks": {
                TASK_UID: {
                    "task_uid": TASK_UID,
                    "repository": REPOSITORY,
                    "issue_number": 3671,
                    "owner_role": "repository_health_engineer",
                    "status": "in_progress",
                    "workflow_phase": "in_progress",
                }
            }}, sort_keys=True
        ))

    def _write_remote_stubs(self):
        workflow = self.script_dir / "github-project-workflow.sh"
        workflow.write_text(
            "#!/usr/bin/env bash\n"
            "set -eu\n"
            "if [[ \"${OASIS7_TRACEABILITY_CONTEXT_ONLY:-0}\" != 1 ]]; then\n"
            "  printf '%s\\n' audit >> \"$OASIS7_CLOSEOUT_MARKER\"\n"
            "  printf '%s\\n' '{\"status\":\"ok\"}'\n"
            "else\n"
            f"  printf '%s\\n' '{{\"status\":\"ok\",\"task_uid\":\"{TASK_UID}\",\"selected_task\":{{\"task_uid\":\"{TASK_UID}\"}}}}'\n"
            "fi\n"
        )
        workflow.chmod(0o755)
        closeout = self.script_dir / "github-project-task.py"
        closeout.write_text(
            "#!/usr/bin/env python3\n"
            "import json\n"
            "import os\n"
            "import sys\n"
            "\n"
            "uid = sys.argv[sys.argv.index('--task-uid') + 1]\n"
            "with open(os.environ['OASIS7_CLOSEOUT_MARKER'], 'a', encoding='utf-8') as handle:\n"
            "    handle.write('closeout\\n')\n"
            "print(json.dumps({'task_uid': uid, 'status': 'deferred', 'issue_url': 'fixture://closeout'}))\n"
        )
        closeout.chmod(0o755)

    def record(self, value):
        return self.write_json("record.json", value)

    def write_json(self, name, value):
        path = self.root / name
        path.write_text(json.dumps(value, sort_keys=True))
        return path

    def run(self, *extra):
        command = [
            str(self.script_dir / "task-closeout.sh"),
            "--role", "repository_health_engineer",
            "--task-uid", TASK_UID,
            "--to-status", "deferred",
            "--verification-profile", "fixture_repository_state",
            *extra,
        ]
        environment = dict(os.environ)
        environment["PM_ROOT_DIR"] = str(self.root)
        environment["OASIS7_CLOSEOUT_MARKER"] = str(self.marker)
        return subprocess.run(command, cwd=self.root, env=environment, text=True, capture_output=True)


class TraceabilityTests(unittest.TestCase):
    def setUp(self):
        self.api = _load_api()
        self.record = _record()
        self.binding = _binding()
        self.readers = FixtureReaders(self.record)

    def leaf(self, record=None, binding=None, readers=None):
        readers = readers or (FixtureReaders(record) if record is not None else self.readers)
        return self.api.validate_leaf(
            record,
            binding or self.binding,
            authority_reader=readers.authority,
            contract_reader=readers.contract,
            source_commit=SOURCE_OID,
        )

    def aggregate(self, candidate, evidence, record=None, readers=None):
        readers = readers or (FixtureReaders(record) if record is not None else self.readers)
        return self.api.validate_aggregate(
            record or self.record,
            candidate,
            evidence,
            authority_reader=readers.authority,
            contract_reader=readers.contract,
            source_commit=SOURCE_OID,
        )

    def refresh_record_binding(self, record):
        record["coordination_ref"]["record_digest"] = _record_digest(record)
        binding = _binding(coordination_ref=deepcopy(record["coordination_ref"]))
        return binding

    def complete_aggregate(self, record=None):
        record = record or self.record
        candidate = _candidate(record)
        second = dict(candidate, source_head_oid="0" * 40)
        first_payload = _evidence_payload(LEAF_UID, candidate)
        second_payload = _evidence_payload(SECOND_LEAF_UID, second)
        first_digest = _evidence_digest(first_payload)
        second_digest = _evidence_digest(second_payload)
        candidate["applicability_matrix"] = [
            _matrix_row("QW2-1", "slot-contract", LEAF_UID, candidate, first_digest),
            _matrix_row("QW2-3", "slot-handoff", SECOND_LEAF_UID, second, second_digest),
        ]
        candidate["equivalence_rules"] = [{
            "source_leaf": {"task_uid": SECOND_LEAF_UID, "source_head_oid": "0" * 40},
            "aggregate_candidate": {"change_id": CHANGE_ID, "tested_tree_oid": candidate["tested_tree_oid"]},
            "allowed_to_differ": ["source_head_oid"],
            "exact_fields": ["change_id", "integration_base_oid", "tested_tree_oid", "configuration_digest", "entry", "environment", "evidence_window"],
            "basis": "approved equivalence of independently reviewed leaf heads",
            "authority_ref": _authority_ref(),
            "approver_role": "producer_system_designer",
            "supporting_evidence_digest": second_digest,
        }]
        evidence = [
            {**first_payload, "evidence_digest": first_digest},
            {**second_payload, "evidence_digest": second_digest},
        ]
        return candidate, evidence

    def assert_blocked_for(self, result, *tokens):
        self.assertEqual(result.get("status"), "blocked", result)
        blockers = "\n".join(str(item) for item in result.get("blockers", []))
        self.assertTrue(all(token in blockers for token in tokens), blockers)

    def test_qw2_1_unresolved_path_fragment_blocks(self):
        record = deepcopy(self.record)
        record["required_obligations"][0]["acceptance_refs"][0]["fragment"] = "missing-fragment"
        result = self.leaf(record, self.refresh_record_binding(record))
        self.assert_blocked_for(result, "doc/engineering/workflow/source-of-truth.md#missing-fragment")

    def test_qw2_2_bare_same_id_consumption_blocks_and_path_refs_pass(self):
        record = deepcopy(self.record)
        record["consumed_clause_refs"] = [
            {"repository": REPOSITORY, "path": "doc/one.md", "fragment": "shared", "clause_id": "shared"},
            {"repository": REPOSITORY, "path": "doc/two.md", "fragment": "shared", "clause_id": "shared"},
        ]
        record["required_obligations"][0]["acceptance_refs"] = ["shared"]
        result = self.leaf(record, self.refresh_record_binding(record))
        self.assert_blocked_for(result, "doc/one.md", "doc/two.md", "path-qualified")

        qualified = deepcopy(self.record)
        qualified["required_obligations"][0]["acceptance_refs"] = [
            {"repository": REPOSITORY, "path": "doc/one.md", "fragment": "shared", "clause_id": "shared"},
            {"repository": REPOSITORY, "path": "doc/two.md", "fragment": "shared", "clause_id": "shared"},
        ]
        self.assertEqual(self.leaf(qualified, self.refresh_record_binding(qualified)).get("status"), "passed")

    def test_qw2_3_required_acceptance_without_handoff_blocks(self):
        record = deepcopy(self.record)
        record["required_obligations"][1]["mapping_slot"] = None
        result = self.leaf(record, self.refresh_record_binding(record))
        self.assert_blocked_for(result, "QW2-3", "mapping_slot")

    def test_qw2_4_terminal_leaves_without_composition_block(self):
        candidate = _candidate(self.record)
        evidence = [
            {"task_uid": LEAF_UID, "status": "passed", "candidate": candidate},
            {"task_uid": SECOND_LEAF_UID, "status": "passed", "candidate": dict(candidate, source_head_oid="0" * 40)},
        ]
        result = self.aggregate(candidate, evidence)
        self.assert_blocked_for(result, "composition evidence missing")

    def test_qw2_5_stale_candidate_field_blocks(self):
        record = deepcopy(self.record)
        candidate, evidence = self.complete_aggregate(record)
        candidate["integration_base_oid"] = "a" * 40
        result = self.aggregate(candidate, evidence, record)
        self.assert_blocked_for(result, "integration_base_oid", "stale")

    def test_qw2_6_old_inflight_binding_survives_new_draft_without_dispatch(self):
        result = self.leaf()
        self.assertEqual(result.get("status"), "passed", result)
        self.assertFalse(result.get("downstream_task_created", False))
        self.assertNotIn("dispatch_request", result)

    def test_qw2_7_revoked_input_blocks_continuation(self):
        record = deepcopy(self.record)
        record["input_contracts"] = [{"contract_id": "engineering-workflow", "revision": "v1.15.2", "eligibility": {"in_flight": False}}]
        result = self.leaf(record, self.refresh_record_binding(record))
        self.assert_blocked_for(result, "in_flight", "revoked")

    def test_qw2_8_blocking_feedback_keeps_leaf_pass_but_blocks_aggregate(self):
        record = deepcopy(self.record)
        record["feedback"] = [{
            "source_locator": "issuecomment-5636949882",
            "receiving_owner": "repository_health_engineer",
            "disposition_authority": "producer_system_designer",
            "decision": "pending",
            "basis": "schema review",
            "authorized_follow_up": "task_3673",
            "affected_consumer": "aggregate",
            "blocking": True,
            "clearance": None,
        }]
        self.assertEqual(self.leaf(record, self.refresh_record_binding(record)).get("status"), "passed")
        candidate, evidence = self.complete_aggregate(record)
        result = self.aggregate(candidate, evidence, record)
        self.assert_blocked_for(result, "feedback", "clearance")

    def test_optional_binding_absence_is_valid_for_ordinary_single_leaf(self):
        binding = _binding()
        binding.pop("coordination_ref")
        binding.pop("change_id")
        result = self.leaf(None, binding)
        self.assertEqual(result.get("status"), "passed", result)

    def test_explicit_matrix_and_equivalence_allow_distinct_leaf_heads(self):
        record = deepcopy(self.record)
        candidate, evidence = self.complete_aggregate(record)
        readers = FixtureReaders(record)
        result = self.aggregate(candidate, evidence, record, readers)
        self.assertEqual(result.get("status"), "passed", result)
        self.assertTrue(readers.authority_calls, "equivalence approval must be read back")
        observed_refs = []
        for args, kwargs in readers.authority_calls:
            observed = args[0] if args else kwargs.get("authority_ref") or kwargs.get("coordination_ref")
            if isinstance(observed, dict):
                observed_refs.append(observed)
        self.assertIn(candidate["equivalence_rules"][0]["authority_ref"], observed_refs)
        self.assertIn(EQUIVALENCE_APPROVAL_COMMENT_ID, [ref["comment_id"] for ref in observed_refs])
        approval_readbacks = [
            result for result in readers.authority_results
            if result["comment"]["id"] == EQUIVALENCE_APPROVAL_COMMENT_ID
        ]
        self.assertEqual(len(approval_readbacks), 1)
        readback = approval_readbacks[0]
        self.assertEqual(readback["reader_kind"], "fixture_authority")
        self.assertEqual(readback["repository"], REPOSITORY)
        self.assertEqual(readback["issue"]["number"], 3671)
        self.assertEqual(readback["issue"]["html_url"], f"https://github.com/{REPOSITORY}/issues/3671")
        self.assertEqual(readback["comment"]["issue_url"], f"https://api.github.com/repos/{REPOSITORY}/issues/3671")
        self.assertEqual(readback["comment"]["user"], {"login": "producer-system-designer"})
        self.assertEqual(readback["comment"]["created_at"], "2026-09-11T00:00:00Z")
        approval_body = json.loads(readback["comment"]["body"])
        self.assertEqual(approval_body["marker"], "oasis7-equivalence-approval")
        self.assertEqual(approval_body["schema"], "oasis7.loop-equivalence-approval/v1")
        self.assertEqual(approval_body["task_uid"], record["task_uid"])
        self.assertEqual(approval_body["change_id"], record["change_id"])
        self.assertEqual(approval_body["approval"], "approved")
        self.assertEqual(
            approval_body["supporting_evidence_digest"],
            candidate["equivalence_rules"][0]["supporting_evidence_digest"],
        )

    def test_missing_matrix_row_blocks_even_with_terminal_leaf_evidence(self):
        record = deepcopy(self.record)
        candidate = _candidate(record)
        payload = _evidence_payload(LEAF_UID, candidate)
        digest = _evidence_digest(payload)
        candidate["applicability_matrix"] = [_matrix_row("QW2-1", "slot-contract", LEAF_UID, candidate, digest)]
        evidence = [{**payload, "evidence_digest": digest}]
        result = self.aggregate(candidate, evidence, record)
        self.assert_blocked_for(result, "applicability_matrix", "slot-handoff")

    def test_matrix_rejects_duplicate_unknown_and_mismatched_rows(self):
        variants = []

        candidate, evidence = self.complete_aggregate()
        duplicate = deepcopy(candidate)
        duplicate["applicability_matrix"].append(deepcopy(duplicate["applicability_matrix"][0]))
        variants.append((duplicate, evidence, ("duplicate", "applicability_matrix")))

        candidate, evidence = self.complete_aggregate()
        unknown = deepcopy(candidate)
        unknown["applicability_matrix"][1]["leaf_task_uid"] = "task_" + "d" * 32
        variants.append((unknown, evidence, ("unknown", "Task UID")))

        candidate, evidence = self.complete_aggregate()
        mismatch = deepcopy(candidate)
        mismatch["applicability_matrix"][0]["tested_tree_oid"] = "9" * 40
        variants.append((mismatch, evidence, ("tested_tree_oid", "matrix")))

        for candidate, evidence, tokens in variants:
            with self.subTest(tokens=tokens):
                self.assert_blocked_for(self.aggregate(candidate, evidence), *tokens)

    def test_matrix_rejects_each_candidate_field_drift(self):
        mutations = {
            "source_head_oid": "9" * 40,
            "integration_base_oid": "8" * 40,
            "tested_tree_oid": "7" * 40,
            "configuration_digest": "sha256:" + "6" * 64,
            "entry": "scripts/pm/other.py",
            "environment": "hosted",
            "evidence_window": {"started_at": "2026-09-11T00:02:00Z", "ended_at": "2026-09-11T00:03:00Z"},
        }
        for field, value in mutations.items():
            with self.subTest(field=field):
                candidate, evidence = self.complete_aggregate()
                candidate["applicability_matrix"][0][field] = value
                self.assert_blocked_for(self.aggregate(candidate, evidence), field, "matrix")

    def test_matrix_rejects_unknown_slot_and_evidence_digest_mismatch(self):
        candidate, evidence = self.complete_aggregate()
        candidate["applicability_matrix"][0]["mapping_slot"] = "slot-unknown"
        self.assert_blocked_for(self.aggregate(candidate, evidence), "unknown", "mapping_slot")

        candidate, evidence = self.complete_aggregate()
        candidate["applicability_matrix"][0]["leaf_evidence_digest"] = "sha256:" + "9" * 64
        self.assert_blocked_for(self.aggregate(candidate, evidence), "evidence_digest", "matrix")

    def test_aggregate_rejects_duplicate_evidence_and_evidence_candidate_drift(self):
        candidate, evidence = self.complete_aggregate()
        duplicate = evidence + [deepcopy(evidence[0])]
        self.assert_blocked_for(self.aggregate(candidate, duplicate), "duplicate", "evidence")

        candidate, evidence = self.complete_aggregate()
        evidence[0]["candidate"]["entry"] = "scripts/pm/other.py"
        self.assert_blocked_for(self.aggregate(candidate, evidence), "entry", "evidence")

    def test_equivalence_requires_actual_leaf_candidate_and_exact_critical_fields(self):
        candidate, evidence = self.complete_aggregate()
        wrong_leaf = deepcopy(candidate)
        wrong_leaf["equivalence_rules"][0]["source_leaf"]["task_uid"] = "task_" + "d" * 32
        self.assert_blocked_for(self.aggregate(wrong_leaf, evidence), "equivalence", "source leaf")

        candidate, evidence = self.complete_aggregate()
        critical = deepcopy(candidate)
        critical["equivalence_rules"][0]["allowed_to_differ"].append("tested_tree_oid")
        self.assert_blocked_for(self.aggregate(critical, evidence), "tested_tree_oid", "exact")

    def test_equivalence_requires_authority_and_supporting_evidence_digest(self):
        candidate, evidence = self.complete_aggregate()
        bad_authority = deepcopy(candidate)
        bad_authority["equivalence_rules"][0]["authority_ref"]["comment_id"] = 999
        self.assert_blocked_for(self.aggregate(bad_authority, evidence), "equivalence", "authority")

        candidate, evidence = self.complete_aggregate()
        bad_digest = deepcopy(candidate)
        bad_digest["equivalence_rules"][0]["supporting_evidence_digest"] = "sha256:" + "9" * 64
        self.assert_blocked_for(self.aggregate(bad_digest, evidence), "equivalence", "digest")

    def test_modified_comparable_fields_cannot_exempt_critical_candidate_data(self):
        record = deepcopy(self.record)
        record["candidate_selection"]["comparable_fields"].remove("tested_tree_oid")
        record["coordination_ref"]["record_digest"] = _record_digest(record)
        candidate, evidence = self.complete_aggregate(record)
        result = self.aggregate(candidate, evidence, record)
        self.assert_blocked_for(result, "tested_tree_oid", "comparable")

    def _authority_variant(self, mutate):
        readers = FixtureReaders(self.record)
        payload = readers.authority()
        mutate(payload)
        readers.authority = lambda *args, **kwargs: payload
        return readers

    def test_authority_reader_receives_exact_bound_identity(self):
        readers = FixtureReaders(self.record)
        result = self.leaf(readers=readers)
        self.assertEqual(result.get("status"), "passed", result)
        self.assertEqual(len(readers.authority_calls), 1)
        args, kwargs = readers.authority_calls[0]
        observed = args[0] if args else kwargs.get("coordination_ref")
        self.assertEqual(observed, self.record["coordination_ref"])

    def test_authority_readback_rejects_independent_issue_identity_tamper(self):
        variants = [
            (lambda payload: payload["issue"].update({"body": "incidental mention " + TASK_UID}), "canonical task_uid"),
            (lambda payload: payload["issue"].update({"number": 999}), "Issue identity"),
            (lambda payload: payload["comment"].update({"id": 999}), "comment identity"),
            (lambda payload: payload["comment"].update({"issue_url": "https://api.github.com/repos/foreign/repo/issues/3671"}), "Issue URL"),
        ]
        for mutate, token in variants:
            with self.subTest(token=token):
                result = self.leaf(readers=self._authority_variant(mutate))
                self.assert_blocked_for(result, token)

    def test_authority_readback_rejects_marker_schema_change_and_change_id_tamper(self):
        variants = [
            (lambda payload: _rewrite_comment_body(payload, lambda body: body.update({"marker": "foreign-marker"})), "marker"),
            (lambda payload: _rewrite_comment_body(payload, lambda body: body.update({"schema": "foreign/v1"})), "schema"),
            (lambda payload: _rewrite_comment_body(payload, lambda body: body.update({"change_id": "other-change"})), "change_id"),
        ]
        for mutate, token in variants:
            with self.subTest(token=token):
                result = self.leaf(readers=self._authority_variant(mutate))
                self.assert_blocked_for(result, token)

    def test_authority_readback_rejects_server_author_and_creation_time_tamper(self):
        variants = [
            (lambda payload: payload["comment"].update({"user": {"login": "attacker"}}), "server author"),
            (lambda payload: payload["comment"].update({"created_at": "2026-09-12T00:00:00Z"}), "creation time"),
        ]
        for mutate, token in variants:
            with self.subTest(token=token):
                result = self.leaf(readers=self._authority_variant(mutate))
                self.assert_blocked_for(result, token)

    def test_authority_readback_rejects_caller_author_and_approval_spoof(self):
        binding = _binding(
            caller_author="coordinator",
            caller_approval="approved",
        )

        def attacker_readback(payload):
            payload["comment"].update({"user": {"login": "attacker"}})

        readers = self._authority_variant(attacker_readback)
        result = self.leaf(binding=binding, readers=readers)
        self.assert_blocked_for(result, "server author")

    def _equivalence_authority_variant(self, record, mutate):
        readers = FixtureReaders(record)
        original_authority = readers.authority

        def authority(*args, **kwargs):
            reference = args[0] if args else kwargs.get("authority_ref") or kwargs.get("coordination_ref")
            result = original_authority(*args, **kwargs)
            if isinstance(reference, dict) and reference.get("comment_id") == EQUIVALENCE_APPROVAL_COMMENT_ID:
                mutate(result)
            return result

        readers.authority = authority
        return readers

    def test_equivalence_approval_readback_rejects_author_or_body_tamper(self):
        variants = [
            (lambda payload: payload["comment"].update({"user": {"login": "attacker"}}), "server author"),
            (lambda payload: _rewrite_comment_body(payload, lambda body: body.update({"basis": "tampered"})), "body_digest"),
        ]
        for mutate, token in variants:
            with self.subTest(token=token):
                record = deepcopy(self.record)
                candidate, evidence = self.complete_aggregate(record)
                readers = self._equivalence_authority_variant(record, mutate)
                result = self.aggregate(candidate, evidence, record, readers)
                self.assert_blocked_for(result, "equivalence", token)

    def test_authority_readback_recomputes_body_record_and_source_digests(self):
        readers = FixtureReaders(self.record)
        readers.comment["body"] += "\ntampered"
        self.assert_blocked_for(self.leaf(readers=readers), "body_digest")

        changed = deepcopy(self.record)
        changed["feedback"].append({"source_locator": "late-mutation"})
        self.assert_blocked_for(self.leaf(changed, readers=readers), "record_digest")

        forged = deepcopy(self.record)
        forged["coordination_ref"]["record_digest"] = "sha256:" + "9" * 64
        forged_readers = FixtureReaders(forged)
        forged_binding = _binding(coordination_ref=deepcopy(forged["coordination_ref"]))
        self.assert_blocked_for(self.leaf(forged, forged_binding, forged_readers), "record_digest")

        stale_source = deepcopy(self.record)
        stale_source["coordination_ref"]["source_commit"] = "f" * 40
        stale_binding = self.refresh_record_binding(stale_source)
        self.assert_blocked_for(self.leaf(stale_source, stale_binding), "source_commit")

    def test_effective_tool_and_record_source_commits_are_distinct(self):
        effective_tool_commit = "1" * 40
        result = self.api.validate_leaf(
            self.record,
            self.binding,
            authority_reader=self.readers.authority,
            contract_reader=self.readers.contract,
            source_commit=effective_tool_commit,
            effective_tool_commit=effective_tool_commit,
            record_source_commit=SOURCE_OID,
        )
        self.assertEqual(result.get("status"), "passed", result)
        self.assertEqual(result.get("effective_tool_commit"), effective_tool_commit)
        self.assertEqual(result.get("record_source_commit"), SOURCE_OID)

    def test_duplicate_coordination_comments_block_exact_readback(self):
        readers = FixtureReaders(self.record)
        payload = readers.authority()
        payload["comments"] = [deepcopy(payload["comment"]), deepcopy(readers.comment)]
        readers.authority = lambda *args, **kwargs: payload
        result = self.leaf(readers=readers)
        self.assert_blocked_for(result, "duplicate", "coordination comment")

    def test_bind_resume_and_doctor_block_before_mutation_with_pinned_helper(self):
        boundary = _load_loop_boundary()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pinned = root / "pinned-tools"
            candidate = root / "candidate-worktree"
            pinned.mkdir()
            candidate.mkdir()
            for command in ("bind", "resume-check", "doctor"):
                marker = candidate / (command + ".mutated")
                loader_calls = []

                def loader(effective_tool_root, source_commit, *, _command=command):
                    loader_calls.append((Path(effective_tool_root).resolve(), source_commit))
                    self.assertEqual(Path(effective_tool_root).resolve(), pinned.resolve())
                    self.assertEqual(source_commit, SOURCE_OID)
                    raise TraceabilityPreflight(f"forced {_command} preflight sentinel")

                def mutation():
                    marker.write_text("MUTATED")

                try:
                    result = boundary.pre_mutation_admission(
                        command,
                        binding=_binding(),
                        target_root=candidate,
                        effective_tool_root=pinned,
                        source_commit=SOURCE_OID,
                        traceability_loader=loader,
                        mutation=mutation,
                    )
                except TraceabilityPreflight as exc:
                    result = {"status": "blocked", "blockers": [str(exc)]}
                with self.subTest(command=command):
                    self.assertEqual(result.get("status"), "blocked", result)
                    self.assertIn(f"forced {command} preflight sentinel", "\n".join(result.get("blockers", [])))
                    self.assertEqual(loader_calls, [(pinned.resolve(), SOURCE_OID)])
                    self.assertFalse(marker.exists(), result)

    def test_loop_main_dispatches_gate_for_bind_resume_and_doctor(self):
        """The facade commands must call the gate, rather than merely defining it."""
        loop = _load_loop_dispatch()

        class NoopReservation:
            def __init__(self, *args, **kwargs):
                self.handle = self

            def __enter__(self):
                return self

            def __exit__(self, *args):
                return False

            def fileno(self):
                return 0

        for command in ("bind", "resume-check", "doctor"):
            with self.subTest(command=command), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                pinned = root / "pinned-tools"
                pinned.mkdir()
                marker = root / "downstream-mutated"
                binding = _binding(task_uid=TASK_UID, policy_commit=SOURCE_OID)
                binding_path = root / "binding.json"
                binding_path.write_text(json.dumps(binding, sort_keys=True))
                task = {
                    "task_uid": TASK_UID,
                    "repository": REPOSITORY,
                    "issue_number": 3671,
                    "owner_role": "repository_health_engineer",
                    "bootstrap_epoch": 1,
                    "loop_binding": deepcopy(binding),
                    "task_branch": "task/engineering-traceability-checker",
                }
                gate_calls = []
                loader_calls = []

                def loader(effective_tool_root, source_commit):
                    loader_calls.append((Path(effective_tool_root).resolve(), source_commit))
                    self.assertEqual(Path(effective_tool_root).resolve(), pinned.resolve())
                    self.assertEqual(source_commit, SOURCE_OID)
                    raise TraceabilityPreflight(f"forced {command} dispatch sentinel")

                def gate(*args, **kwargs):
                    observed_command = args[0] if args else kwargs.get("command")
                    gate_calls.append((observed_command, kwargs))
                    self.assertEqual(observed_command, command)
                    self.assertEqual(Path(kwargs["effective_tool_root"]).resolve(), pinned.resolve())
                    self.assertEqual(kwargs["source_commit"], SOURCE_OID)
                    self.assertTrue(callable(kwargs["traceability_loader"]))
                    kwargs["traceability_loader"](kwargs["effective_tool_root"], kwargs["source_commit"])

                def downstream_validate(*args, **kwargs):
                    marker.write_text(command)
                    return {"status": "passed", "blockers": []}

                def fake_run(command_args, *args, **kwargs):
                    command_text = " ".join(str(part) for part in command_args)
                    if "workflow-next.py" in command_text:
                        marker.write_text(command)
                    return subprocess.CompletedProcess(command_args, 0, stdout='{"status":"passed"}\n', stderr="")

                def fake_check_output(command_args, *args, **kwargs):
                    command_text = " ".join(str(part) for part in command_args)
                    if "github-project-task.py" in command_text:
                        marker.write_text(command)
                        return '{"status":"bound"}\n'
                    raise AssertionError(f"unexpected command output request: {command_text}")

                argv = [
                    "loop.py",
                    command,
                    "--repo-root", str(root),
                    "--tool-root", str(pinned),
                    "--task-uid", TASK_UID,
                    "--manual-request-ref", "issuecomment-5636639918",
                    "--json",
                ]
                if command == "bind":
                    argv.extend(["--loop-binding", str(binding_path)])

                with patch.object(loop, "pre_mutation_admission", gate, create=True), \
                        patch.object(loop, "_traceability_adapter", lambda effective_root, target_root, bound, commit: loader(effective_root, commit)), \
                        patch.object(loop, "load_task", return_value=task), \
                        patch.object(loop, "validate_task", side_effect=downstream_validate), \
                        patch.object(loop, "common_dir", return_value=root), \
                        patch.object(loop, "Reservation", NoopReservation), \
                        patch.object(loop, "recovery_status", return_value={"pending_actions": []}), \
                        patch.object(loop, "record_action"), \
                        patch.object(loop.subprocess, "run", side_effect=fake_run), \
                        patch.object(loop.subprocess, "check_output", side_effect=fake_check_output), \
                        patch.object(loop.sys, "argv", argv), \
                        redirect_stdout(io.StringIO()):
                    try:
                        return_code = loop.main()
                    except TraceabilityPreflight as exc:
                        return_code = 2
                        self.assertIn(f"forced {command} dispatch sentinel", str(exc))

                self.assertEqual(len(gate_calls), 1, gate_calls)
                self.assertEqual(loader_calls, [(pinned.resolve(), SOURCE_OID)])
                self.assertEqual(return_code, 2)
                self.assertFalse(marker.exists(), f"{command} reached downstream mutation")

    def test_closeout_aggregate_omitted_record_blocks_before_remote_mutation(self):
        fixture = CloseoutFixture()
        self.addCleanup(fixture.tmp.cleanup)
        result = fixture.run("--traceability-mode", "aggregate")
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        self.assertIn("coordinating record", output)
        self.assertFalse(fixture.marker.exists(), output)

    def test_closeout_cannot_downgrade_aggregate_context_to_leaf(self):
        fixture = CloseoutFixture()
        self.addCleanup(fixture.tmp.cleanup)
        record = fixture.record(self.record)
        result = fixture.run(
            "--traceability-mode", "leaf",
            "--traceability-record", str(record),
        )
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        self.assertIn("cannot downgrade", output)
        self.assertFalse(fixture.marker.exists(), output)

    def test_closeout_failed_aggregate_preflight_blocks_before_remote_mutation(self):
        fixture = CloseoutFixture()
        self.addCleanup(fixture.tmp.cleanup)
        mapping_path = fixture.root / ".pm" / "github-project-sync" / "tasks.json"
        mapping = json.loads(mapping_path.read_text())
        mapping["tasks"][TASK_UID]["loop_binding"] = {"policy_commit": SOURCE_OID}
        mapping_path.write_text(json.dumps(mapping, sort_keys=True))
        record = fixture.record(self.record)
        candidate = fixture.write_json("candidate.json", _candidate(self.record))
        (fixture.script_dir / "loop_traceability.py").write_text(
            "def validate_aggregate(*args, **kwargs):\n"
            "    return {'status': 'blocked', 'blockers': ['forced aggregate preflight sentinel']}\n"
        )
        result = fixture.run(
            "--traceability-mode", "aggregate",
            "--traceability-record", str(record),
            "--traceability-candidate", str(candidate),
        )
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        self.assertIn("forced aggregate preflight sentinel", output)
        self.assertFalse(fixture.marker.exists(), output)

    def test_closeout_rejects_valid_record_for_another_selected_task(self):
        fixture = CloseoutFixture()
        self.addCleanup(fixture.tmp.cleanup)
        other = deepcopy(self.record)
        other["task_uid"] = "task_" + "b" * 32
        other["coordination_ref"]["issue_number"] = 3672
        other["coordination_ref"]["comment_id"] = 5636906115
        other["coordination_ref"]["record_digest"] = _record_digest(other)
        record = fixture.record(other)
        candidate, evidence = self.complete_aggregate(other)
        candidate_payload = fixture.write_json(
            "candidate.json", {"candidate": candidate, "evidence": evidence}
        )
        result = fixture.run(
            "--traceability-mode", "aggregate",
            "--traceability-record", str(record),
            "--traceability-candidate", str(candidate_payload),
        )
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        self.assertIn("selected task", output)
        self.assertFalse(fixture.marker.exists(), output)

    def test_closeout_ordinary_unbound_leaf_remains_eligible(self):
        fixture = CloseoutFixture()
        self.addCleanup(fixture.tmp.cleanup)
        result = fixture.run()
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 0, output)
        self.assertEqual(fixture.marker.read_text().splitlines(), ["audit", "closeout", "audit"], output)

    def test_reverse_consumer_view_is_readonly_and_authority_bounded(self):
        result = self.api.reverse_consumers(
            _contract_ref(),
            authority_reader=self.readers.authority,
            reader_kind="fixture_authority",
        )
        self.assertEqual(result.get("status"), "passed", result)
        self.assertIsInstance(result.get("consumers"), list)
        self.assertEqual(result.get("reader_kind"), "fixture_authority", result)
        self.assertTrue(result.get("local_live_admission_required"), result)
        self.assertNotIn("mutation", result)
        self.assertNotIn("dispatch_request", result)

    def test_immutable_source_reader_uses_frozen_anchor_and_distinct_content_digest(self):
        """The source proof must resolve committed bytes and exact anchors."""
        repository_root = HERE.parent.parent
        frozen = self.api.ImmutableSourceReader(repository_root, SOURCE_OID)
        published_shape = {
            "repository": REPOSITORY,
            "path": "doc/engineering/workflow/source-of-truth.md",
            "fragment": "traceability-record-contract",
            "contract_id": "engineering-workflow",
            "revision": 1,
        }
        live_source = frozen(published_shape)
        self.assertEqual(live_source.get("status"), "passed", live_source)
        self.assertEqual(live_source.get("source_commit"), SOURCE_OID)

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "-C", str(root), "init", "-q"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "traceability@example.invalid"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "traceability-test"], check=True)
            document = root / "contract.md"
            exact = '<a id="traceability-record-contract"></a>\n# Contract\n'
            document.write_text(exact, encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "contract.md"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "-qm", "exact contract anchor"], check=True)
            commit = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
            raw_digest = "sha256:" + hashlib.sha256(exact.encode("utf-8")).hexdigest()
            reference = {
                "repository": REPOSITORY,
                "path": "contract.md",
                "fragment": "traceability-record-contract",
                "source_digest": raw_digest,
            }
            reader = self.api.ImmutableSourceReader(root, commit)
            result = reader(reference)
            self.assertEqual(result.get("status"), "passed", result)
            self.assertEqual(result.get("source_digest"), raw_digest)

            tampered = dict(reference)
            tampered["source_digest"] = "sha256:" + "9" * 64
            with self.assertRaises(ValueError) as digest_error:
                reader(tampered)
            self.assertIn("digest mismatch", str(digest_error.exception))

            document.write_text("traceability-record-contract is only mentioned\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "contract.md"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "-qm", "remove anchor"], check=True)
            missing_commit = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
            with self.assertRaises(ValueError) as anchor_error:
                self.api.ImmutableSourceReader(root, missing_commit)(reference)
            self.assertIn("unresolved immutable contract fragment", str(anchor_error.exception))


if __name__ == "__main__":
    unittest.main()
