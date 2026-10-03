#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import hashlib
import json
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
PATH = ROOT / "scripts/pm/first_activation.py"
SPEC = importlib.util.spec_from_file_location("first_activation_under_test", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
TASK_HELPER_SPEC = importlib.util.spec_from_file_location(
    "first_activation_task_helper_test", ROOT / "scripts/pm/github-project-task.py"
)
assert TASK_HELPER_SPEC and TASK_HELPER_SPEC.loader
TASK_HELPER = importlib.util.module_from_spec(TASK_HELPER_SPEC)
TASK_HELPER_SPEC.loader.exec_module(TASK_HELPER)

UID = "task_" + "a" * 32
BASE = "1" * 40
HEAD = "2" * 40
SHA = "sha256:" + "a" * 64
ISSUE_URL = "https://api.github.com/repos/eng-cc/oasis7/issues/4269"


def _comment(comment_id: int, body: str, second: int) -> dict:
    return {
        "id": comment_id,
        "issue_url": ISSUE_URL,
        "body": body,
        "user": {"login": "human"},
        "created_at": f"2026-10-03T00:00:{second:02d}Z",
    }


def review_evidence(value: dict, packet_digests: dict[str, str] | None = None) -> tuple[list[dict], dict[str, dict]]:
    packet_digests = packet_digests or {}
    auth_body = "User authorizes oasis7_wasm_executor wasmtime 48.0.3 to 48.0.4 effective_package_dependency_floor"
    value["authorization"]["body_sha256"] = "sha256:" + hashlib.sha256(auth_body.encode()).hexdigest()
    dispatch_body = "Authorized implementation slice implementation-checker: .github/workflows/rust.yml"
    dispatch_digest = "sha256:" + hashlib.sha256(dispatch_body.encode()).hexdigest()
    plan = {
        "schema": MODULE.FIRST_REVIEW_PLAN_SCHEMA,
        "task_uid": UID,
        "issue_number": 4269,
        "bootstrap_epoch": value["bootstrap_epoch"],
        "snapshot_sha256": value["snapshot_sha256"],
        "request_sha256": value["request_sha256"],
        "acceptance_sha256": value["acceptance_sha256"],
        "raw_primary_package": value["raw_primary_package"],
        "effective_primary_package": value["effective_primary_package"],
        "dependency": value["dependency"],
        "base_oid": BASE,
        "head_oid": HEAD,
        "authorization": value["authorization"],
        "workflow": {key: value["workflow"][key] for key in
                     ("id", "path", "ref", "sha", "file_sha256")},
        "workflow_change_paths": value["workflow"]["change_paths"],
        "implementation_slices": [{
            "role": "repository_health_engineer",
            "slice_id": "implementation-checker",
            "dispatch_comments": [{"comment_id": 2, "body_sha256": dispatch_digest}],
            "write_scope_paths": [".github/workflows/rust.yml"],
        }],
        "review_slices": [
            {"role": "qa_engineer", "slice_id": "first-qa",
             "packet_sha256": packet_digests.get("qa_engineer", SHA),
             "write_scope": MODULE.FIRST_REVIEW_WRITE_SCOPE},
            {"role": "repository_health_engineer", "slice_id": "first-rh",
             "packet_sha256": packet_digests.get("repository_health_engineer", SHA),
             "write_scope": MODULE.FIRST_REVIEW_WRITE_SCOPE},
        ],
    }
    plan_body = MODULE.canonical_first_review_plan_body(plan)
    plan_sha = "sha256:" + hashlib.sha256(plan_body.encode()).hexdigest()
    comments = [
        _comment(1, auth_body, 1),
        _comment(2, dispatch_body, 2),
        _comment(3, plan_body, 3),
    ]
    envelopes: dict[str, dict] = {}
    for return_id, review_id, envelope_role, plan_role, slice_id in (
        (4, 5, "repository_health", "repository_health_engineer", "first-rh"),
        (6, 7, "qa", "qa_engineer", "first-qa"),
    ):
        returned = {
            "schema": MODULE.FIRST_REVIEW_RETURN_SCHEMA,
            "task_uid": UID,
            "issue_number": 4269,
            "role": plan_role,
            "slice_id": slice_id,
            "base_oid": BASE,
            "head_oid": HEAD,
            "review_plan_sha256": plan_sha,
            "admitted_packet_sha256": packet_digests.get(plan_role, SHA),
            "return_status": "completed",
            "disposition": "no_findings",
            "findings": [],
            "unresolved_findings": [],
            "residual_risk": [],
        }
        return_body = MODULE.canonical_first_review_return_body(returned)
        return_sha = "sha256:" + hashlib.sha256(return_body.encode()).hexdigest()
        typed = {
            "schema": MODULE.REVIEW_SCHEMA,
            "task_uid": UID,
            "issue_number": 4269,
            "role": envelope_role,
            "slice_id": slice_id,
            "base_oid": BASE,
            "head_oid": HEAD,
            "workflow_change_paths": value["workflow"]["change_paths"],
            "review_plan_sha256": plan_sha,
            "admitted_packet_sha256": packet_digests.get(plan_role, SHA),
            "return_sha256": return_sha,
            "return_status": "completed",
            "findings": [],
            "unresolved_findings": [],
        }
        envelope_body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(typed).decode() + "\n"
        value["reviews"][envelope_role]["comment_id"] = review_id
        value["reviews"][envelope_role]["body_sha256"] = "sha256:" + hashlib.sha256(envelope_body.encode()).hexdigest()
        envelopes[envelope_role] = typed
        comments.extend((_comment(return_id, return_body, return_id),
                         _comment(review_id, envelope_body, review_id)))
    return comments, envelopes


def overlay() -> dict:
    return {
        "schema": MODULE.OVERLAY_SCHEMA,
        "task_uid": UID,
        "issue_number": 4269,
        "bootstrap_epoch": 1,
        "snapshot_sha256": SHA,
        "request_sha256": SHA,
        "acceptance_sha256": SHA,
        "raw_primary_package": {"present": False, "value": None},
        "effective_primary_package": "oasis7_wasm_executor",
        "mode": "dependency_floor_update",
        "dependency": {
            "name": "wasmtime",
            "base_requirement": "48.0.3",
            "head_requirement": "48.0.4",
        },
        "base_oid": BASE,
        "head_oid": HEAD,
        "authorization": {"comment_id": 1, "body_sha256": SHA, "scope": "effective_package_dependency_floor"},
        "reviews": {
            "repository_health": {"comment_id": 2, "body_sha256": SHA},
            "qa": {"comment_id": 3, "body_sha256": SHA},
        },
        "workflow": {
            "id": 123456,
            "path": ".github/workflows/rust.yml",
            "ref": "refs/heads/codex/test",
            "sha": HEAD,
            "file_sha256": SHA,
            "change_paths": [
                {"path": ".github/workflows/rust.yml", "base_sha256": SHA, "head_sha256": SHA}
            ],
        },
    }


class OverlayShapeTests(unittest.TestCase):
    def test_request_identity_uses_canonical_issue_title_reconstruction(self):
        self.assertEqual(
            TASK_HELPER.normalize_issue_title("[PM] Upgrade Wasmtime to patched 48.0.4"),
            "Upgrade Wasmtime to patched 48.0.4",
        )
        self.assertEqual(TASK_HELPER.normalize_issue_title("[pm] unchanged"), "[pm] unchanged")
        self.assertEqual(TASK_HELPER.normalize_issue_title("[PM]"), "[PM]")

    def test_valid_overlay_returns_only_bound_selector_fields(self):
        parsed = MODULE.validate_overlay(overlay(), task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        self.assertEqual(parsed["mode"], "dependency_floor_update")
        self.assertEqual(parsed["effective_primary_package"], "oasis7_wasm_executor")
        self.assertEqual(parsed["workflow_change_paths"][0]["path"], ".github/workflows/rust.yml")
        self.assertTrue(parsed["validation_only"])

    def test_exact_first_activation_path_set_and_rejects_business_manifest(self):
        expected_paths = {
            "doc/engineering/workflow/source-of-truth.md",
            "doc/.governance/document-corpus/objects/48/4840d720cacf3f7d531e8a361fc146277494b75857c9bd904bbc4b0f700c6f41.json",
            ".github/workflows/rust.yml",
            "scripts/ci-tests.sh",
            "scripts/ci-required-scope-audit-contract.test.sh",
            "scripts/ci-required-capability-test-inventory.tsv",
            "scripts/pm/check-cargo-package-scope",
            "scripts/pm/check-cargo-package-scope.test.py",
            "scripts/pm/first_activation.py",
            "scripts/pm/first_activation.test.py",
            "scripts/pm/github-project-task.py",
            "scripts/pm/pr-lifecycle-gate.py",
            "scripts/prepare-task-pr.sh",
            "scripts/prepare-task-pr.test.sh",
        }
        self.assertEqual(MODULE.FIRST_ACTIVATION_REVIEWED_PATHS, expected_paths)
        self.assertTrue(MODULE._dispatch_scope_mentions(
            "scripts/pm/check-cargo-package-scope.test.py",
            "verification check-cargo-package-scope.test.py",
        ))
        self.assertFalse(MODULE._dispatch_scope_mentions(
            "scripts/pm/check-cargo-package-scope.test.py", "verification unrelated.test.py",
        ))
        row = overlay()
        row["workflow"]["change_paths"] = [
            {"path": path, "base_sha256": SHA, "head_sha256": SHA}
            for path in sorted(expected_paths)
        ]
        parsed = MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        self.assertEqual({item["path"] for item in parsed["workflow_change_paths"]}, expected_paths)

        row["workflow"]["change_paths"].append({
            "path": "crates/oasis7_wasm_executor/Cargo.toml",
            "base_sha256": SHA,
            "head_sha256": SHA,
        })
        row["workflow"]["change_paths"].sort(key=lambda item: item["path"])
        with self.assertRaises(MODULE.OverlayError):
            MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                    base_oid=BASE, head_oid=HEAD)

    def test_rejects_uid_head_mode_and_raw_primary_drift(self):
        for mutate in (
            lambda row: row.update(task_uid="task_" + "b" * 32),
            lambda row: row.update(head_oid="3" * 40),
            lambda row: row.update(mode="auto"),
            lambda row: row.update(raw_primary_package={"present": True, "value": "oasis7"}),
        ):
            row = overlay()
            mutate(row)
            with self.subTest(row=row):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                            base_oid=BASE, head_oid=HEAD,
                                            expected_raw_primary={"present": False, "value": None})

    def test_rejects_duplicate_or_escaping_workflow_paths(self):
        for path in ("../outside", "/absolute", "scripts//bad", "crates/other/src/lib.rs",
                     "Cargo.lock", ".cargo/config.toml", "crates/other/Cargo.toml"):
            row = overlay()
            row["workflow"]["change_paths"][0]["path"] = path
            row["workflow"]["path"] = path
            with self.subTest(path=path):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                            base_oid=BASE, head_oid=HEAD)

    def test_rejects_non_patch_raise_and_noncanonical_requirement(self):
        for head_req in ("48.1.0", "48.0.04", "48.0.4-alpha"):
            row = overlay()
            row["dependency"]["head_requirement"] = head_req
            with self.subTest(head_req=head_req):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                            base_oid=BASE, head_oid=HEAD)

    def test_live_dispatch_provenance_requires_exact_workflow_and_run_attempt(self):
        parsed = MODULE.validate_overlay(overlay(), task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        run = {
            "id": 99,
            "run_attempt": 1,
            "event": "workflow_dispatch",
            "workflow_id": parsed["workflow_id"],
            "path": parsed["workflow_path"],
            "head_branch": "codex/test",
            "head_sha": HEAD,
            "repository": {"full_name": "eng-cc/oasis7"},
            "status": "completed",
            "conclusion": "success",
        }
        MODULE.validate_workflow_run_provenance(
            run, repository="eng-cc/oasis7", overlay=parsed, run_id=99, run_attempt=1,
            event_name="workflow_dispatch", workflow_ref="refs/heads/codex/test",
            workflow_sha=HEAD,
        )
        for key, value in (("id", 100), ("run_attempt", 2), ("workflow_id", 7),
                           ("head_sha", "3" * 40), ("event", "pull_request"),
                           ("conclusion", "failure")):
            changed = dict(run)
            changed[key] = value
            with self.subTest(key=key):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_workflow_run_provenance(
                        changed, repository="eng-cc/oasis7", overlay=parsed, run_id=99,
                        run_attempt=1, event_name="workflow_dispatch",
                        workflow_ref="refs/heads/codex/test", workflow_sha=HEAD,
                    )

    def test_referenced_review_evidence_requires_canonical_typed_records(self):
        value = overlay()
        comments, review_records = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)
        bad = dict(review_records["qa"])
        bad["head_oid"] = BASE
        bad_body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(bad).decode() + "\n"
        comments[-1] = {**comments[-1], "body": bad_body}
        value["reviews"]["qa"]["body_sha256"] = "sha256:" + hashlib.sha256(bad_body.encode()).hexdigest()
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)

    def test_first_activation_plan_binds_all_dispatch_comments_and_path_scope(self):
        value = overlay()
        comments, _ = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)
        plan_comment, payload, plan_sha = MODULE._plan_comment(
            comments, ISSUE_URL, MODULE.validate_overlay(
                value, task_uid=UID, issue_number=4269, base_oid=BASE, head_oid=HEAD,
            ),
        )
        self.assertEqual(plan_sha, comments[2]["body"] and "sha256:" + hashlib.sha256(comments[2]["body"].encode()).hexdigest())
        wrapper = {"_overlay": parsed["_overlay"]}
        checked = MODULE._validate_first_review_plan(payload, wrapper, repository="eng-cc/oasis7",
                                                     issue_number=4269, comments=comments)
        self.assertEqual(checked["implementation_slices"][0]["dispatch_comments"][0]["comment_id"], 2)
        bad = json.loads(json.dumps(payload))
        bad["implementation_slices"][0]["dispatch_comments"].append(
            {"comment_id": 2, "body_sha256": SHA})
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_first_review_plan(bad, wrapper, repository="eng-cc/oasis7",
                                               issue_number=4269, comments=comments)

    def test_plan_history_selects_exact_head_and_rejects_duplicate_or_wrong_identity(self):
        value = overlay()
        comments, _ = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        stale = json.loads(comments[2]["body"].split("\n", 1)[1])
        stale["head_oid"] = BASE
        stale_body = MODULE.FIRST_REVIEW_PLAN_MARKER + "\n" + MODULE._canonical(stale).decode() + "\n"
        historical = _comment(30, stale_body, 30)
        selected, _, selected_digest = MODULE._plan_comment(
            [*comments, historical], ISSUE_URL, parsed,
        )
        self.assertEqual(selected["id"], comments[2]["id"])
        self.assertEqual(selected_digest, "sha256:" + hashlib.sha256(comments[2]["body"].encode()).hexdigest())
        with self.assertRaises(MODULE.OverlayError):
            MODULE._plan_comment([*comments, comments[2] | {"id": 31}], ISSUE_URL, parsed)
        with self.assertRaises(MODULE.OverlayError):
            MODULE._plan_comment([historical], ISSUE_URL, parsed)

    def test_typed_return_digest_must_resolve_to_matching_exact_head_record(self):
        value = overlay()
        comments, records = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        bad = dict(records["qa"])
        bad["head_oid"] = BASE
        bad_body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(bad).decode() + "\n"
        comments[-1] = {**comments[-1], "body": bad_body}
        value["reviews"]["qa"]["body_sha256"] = "sha256:" + hashlib.sha256(bad_body.encode()).hexdigest()
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)

    def test_local_first_activation_checks_actual_packet_bytes_and_project_binding(self):
        import tempfile

        value = overlay()
        packet_values = {}
        raw_packets = {}
        for role, slice_id in (("qa_engineer", "first-qa"),
                               ("repository_health_engineer", "first-rh")):
            packet = {
                "schema": "oasis7-subagent-task-packet/v1",
                "identity": {
                    "task_uid": UID,
                    "head": HEAD,
                    "base_sha": BASE,
                    "base_binding": "immutable_oid",
                    "repository": "eng-cc/oasis7",
                    "issue_url": ISSUE_URL,
                    "project_item_id": "project-item",
                    "branch": "codex/test",
                },
                "slice": {
                    "role": role,
                    "slice_id": slice_id,
                    "slice_type": "professional_review",
                    "write_scope": MODULE.FIRST_REVIEW_WRITE_SCOPE,
                },
            }
            packet["packet_digest"] = MODULE._packet_digest(packet)
            raw = (json.dumps(packet, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()
            raw_packets[role] = raw
            packet_values[role] = "sha256:" + hashlib.sha256(raw).hexdigest()
        comments, _ = review_evidence(value, packet_values)
        with tempfile.TemporaryDirectory() as temp:
            packet_dir = pathlib.Path(temp) / ".pm" / "scratch" / UID / "slice-packets"
            packet_dir.mkdir(parents=True)
            (packet_dir / "first-qa.json").write_bytes(raw_packets["qa_engineer"])
            (packet_dir / "first-rh.json").write_bytes(raw_packets["repository_health_engineer"])
            MODULE._validate_local_review_artifacts(
                pathlib.Path(temp), "eng-cc/oasis7", UID, "project-item", value, comments,
            )
            (packet_dir / "first-rh.json").write_bytes(raw_packets["repository_health_engineer"] + b" ")
            with self.assertRaises(MODULE.OverlayError):
                MODULE._validate_local_review_artifacts(
                    pathlib.Path(temp), "eng-cc/oasis7", UID, "project-item", value, comments,
                )

    def test_activation_record_binds_exact_overlay_and_reconstructed_tier(self):
        parsed = MODULE.validate_overlay(overlay(), task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        parsed["overlay_comment_id"] = 44
        parsed["overlay_sha256"] = SHA
        proof = {
            "schema": "oasis7-cargo-dependency-floor-required-tier-proof/v1",
            "workflow_run_id": 99,
            "run_attempt": 1,
            "required_gate_job_id": 101,
            "required_gate_check_run_id": 102,
            "required_gate_check_app_id": 103,
            "inventory_digest": SHA,
            "obligation_count": 4,
            "workflow_logs_sha256": SHA,
        }
        payload = MODULE._activation_payload(parsed, proof, 99, 1)
        self.assertEqual((99, 1, proof), MODULE._validate_activation_payload(payload, parsed))
        payload["workflow"]["run_attempt"] = 2
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_activation_payload(payload, parsed)

    def test_issue_only_activation_reader_reuses_exact_live_proof_without_project_claim(self):
        raw = overlay()
        parsed = MODULE.validate_overlay(raw, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        parsed["overlay_comment_id"] = 44
        parsed["overlay_sha256"] = "sha256:" + hashlib.sha256(
            MODULE.canonical_overlay_body(raw).encode(),
        ).hexdigest()
        with mock.patch.object(MODULE, "read_issue_overlay", return_value=parsed) as issue_reader, \
             mock.patch.object(MODULE, "_read_live_activation_proof", return_value={
                 **parsed, "activated": True, "required_tier_proof": {"workflow_run_id": 99},
             }) as proof_reader:
            result = MODULE.read_issue_activation(
                pathlib.Path("."), "eng-cc/oasis7", UID, BASE, HEAD, client=object(),
            )
        issue_reader.assert_called_once()
        proof_reader.assert_called_once()
        self.assertTrue(result["activated"])
        self.assertTrue(result["validation_only"])
        self.assertFalse(result["project_membership_verified"])
        self.assertNotIn("project_item_id", result)


class RequiredRunLeafProofTests(unittest.TestCase):
    HEADER = "historical_required_location\ttest_paths\tnew_required_selection\tlegacy_required_coverage\tfull_full_core_full_support\n"

    @staticmethod
    def _git(root: pathlib.Path, *args: str) -> str:
        return subprocess.run(["git", "-C", str(root), *args], check=True,
                              text=True, capture_output=True).stdout.strip()

    def _fixture(self, temp: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path, str, str, dict, str]:
        root = temp / "repository"
        root.mkdir()
        self._git(root, "init", "-b", "codex/test")
        self._git(root, "config", "user.name", "Activation Test")
        self._git(root, "config", "user.email", "activation-test@example.invalid")
        (root / "scripts/pm").mkdir(parents=True)
        (root / "scripts").mkdir(exist_ok=True)
        base_row = (
            "run_workflow_governance_baseline_contract_tests\t"
            "scripts/ci-required-baseline-routing.test.sh,scripts/ci-required-domain-isolation.test.sh\t"
            "workflow_governance\tbaseline retained\tall full groups\n"
        )
        inventory_path = root / "scripts/ci-required-capability-test-inventory.tsv"
        inventory_path.write_text(self.HEADER + base_row, encoding="utf-8")
        (root / "scripts/plan-rust-required-scope.py").write_text(
            "print('scope=full')\n", encoding="utf-8",
        )
        test_inventory = {
            "unit_specs": [
                {
                    "unit_id": "required_gate_baseline",
                    "obligation_set": ["required_gate_baseline:000:required-domain-selector-validation"],
                    "unit_contract": {"commands_and_obligations": [
                        "document-corpus-v3-check", "product-doc-changed-range", "product-doc-full-corpus",
                        "workflow-process-identity", "lint-skills", "windows-paths", "script-executable-bits",
                        "workflow-impact-projection-consumer", "cargo-package-scope-and-profile-completion",
                        "unified-world-terminology", "rust-file-size-regression-and-check",
                        "required-domain-selector-validation", "scripts/ci-required-baseline-routing.test.sh",
                        "scripts/ci-required-domain-isolation.test.sh", "scripts/check-rust-file-size.test.sh",
                        "scripts/check-rust-file-size.sh", "scripts/unified-world-code-terminology-scan.sh",
                    ]},
                },
                {
                    "unit_id": "oasis7_required",
                    "obligation_set": ["oasis7_required:000:cargo-test"],
                    "unit_contract": {"commands_and_obligations": [
                        "cargo test -p oasis7 --tests --features test_tier_required",
                    ]},
                },
                {
                    "unit_id": "workflow_governance",
                    "obligation_set": ["workflow_governance:000:baseline-routing-test"],
                    "unit_contract": {"commands_and_obligations": [
                        "scripts/ci-required-baseline-routing.test.sh",
                    ]},
                },
                {
                    "unit_id": "product_document:doc/product/example.prd.md",
                    "obligation_set": ["product_document:doc/product/example.prd.md"],
                    "unit_contract": {"commands_and_obligations": [
                        "product-document:doc/product/example.prd.md",
                    ]},
                },
            ],
            "planner_output": {"scope": "full"},
            "planner_inventory_issuer": {"inventory_digest": SHA},
        }
        module_source = (
            "def build_required_inventory(*args, **kwargs):\n"
            f"    return {test_inventory!r}\n"
        )
        (root / "scripts/pm/ci_required_inventory.py").write_text(module_source, encoding="utf-8")
        (root / ".github").mkdir()
        (root / ".github/workflows").mkdir()
        (root / ".github/workflows/rust.yml").write_text("name: Rust\n", encoding="utf-8")
        (root / "scripts/ci-tests.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
        self._git(root, "add", ".")
        self._git(root, "commit", "-m", "trusted base fixture")
        base_oid = self._git(root, "rev-parse", "HEAD")
        (root / "scripts/pm/first_activation.test.py").write_text("# candidate test\n", encoding="utf-8")
        inventory_path.write_text(
            self.HEADER + base_row
            + "run_workflow_governance_operational_contract_tests\t"
            + "scripts/pm/first_activation.test.py\tworkflow_governance\tadditive test\tall full groups\n",
            encoding="utf-8",
        )
        (root / "scripts/ci-tests.sh").write_text("#!/usr/bin/env bash\n# candidate runner\n", encoding="utf-8")
        self._git(root, "add", ".")
        self._git(root, "commit", "-m", "candidate head fixture")
        head_oid = self._git(root, "rev-parse", "HEAD")
        base_worktree = temp / "trusted-base"
        self._git(root, "worktree", "add", "--detach", str(base_worktree), base_oid)
        proof_overlay = {
            "task_uid": UID, "issue_number": 4269, "base_oid": base_oid, "head_oid": head_oid,
            "workflow_id": 123456, "workflow_path": ".github/workflows/rust.yml",
            "workflow_ref": "refs/heads/codex/test", "workflow_sha": head_oid,
            "workflow_change_paths": [
                {"path": "scripts/ci-required-capability-test-inventory.tsv"},
                {"path": "scripts/pm/first_activation.test.py"},
            ],
        }
        logs = [
            "required-gate\tRun required test tier\t2026-10-03T00:00:01Z\t+ ./scripts/doc-governance-check.sh --full-corpus",
            "required-gate\tRun required test tier\t2026-10-03T00:00:02Z\tproduct-doc-content: checked 0: reason=no new or substantive product-document changes in selected range",
            "required-gate\tRun required test tier\t2026-10-03T00:00:03Z\tdocument-corpus-inventory-check: OK",
            "required-gate\tRun required test tier\t2026-10-03T00:00:04Z\tworkflow-process-identity-check: PASS",
            "required-gate\tRun required test tier\t2026-10-03T00:00:05Z\tproduct-doc-content: OK (full-corpus checked 4 current-tree product documents)",
            "required-gate\tRun required test tier\t2026-10-03T00:00:06Z\t+ ./scripts/lint-skills.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:07Z\tlint-skills: OK (8 default skill entrypoints, 2 library skill entries checked)",
            "required-gate\tRun required test tier\t2026-10-03T00:00:08Z\t+ ./scripts/check-windows-paths.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:09Z\tok: checked 123 tracked paths for Windows checkout compatibility",
            "required-gate\tRun required test tier\t2026-10-03T00:00:10Z\t+ bash ./scripts/check-script-executable-bits.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:11Z\tok: required release scripts are tracked and executable",
            f"required-gate\tRun required test tier\t2026-10-03T00:00:12Z\t+ python3 - /tmp/impact-projection.json {root} {root}/scripts",
            "required-gate\tRun required test tier\t2026-10-03T00:00:13Z\tworkflow-impact-projection-consumer: verified status",
            f"required-gate\tRun required test tier\t2026-10-03T00:00:14Z\t+ python3 /tmp/trusted-check-cargo-package-scope --repo-root {root} --base {base_oid} --head {head_oid} --primary-package auto --policy {root}/.pm/cargo-package-scope-policy.json --json",
            'required-gate\tRun required test tier\t2026-10-03T00:00:15Z\t{"status":"rejected","reason":"ambiguous_package_attribution"}',
            "required-gate\tRun required test tier\t2026-10-03T00:00:16Z\tfirst-activation trusted checker observation: exit=1 reason=ambiguous_package_attribution status=failed",
            f"required-gate\tRun required test tier\t2026-10-03T00:00:17Z\t+ python3 {root}/scripts/pm/check-cargo-package-scope --repo-root {root} --base {base_oid} --head {head_oid} --first-activation-task-uid {UID} --policy {root}/.pm/cargo-package-scope-policy.json --json",
            f'required-gate\tRun required test tier\t2026-10-03T00:00:18Z\t{{"status":"allowed","primary_package":"oasis7","mode":"dependency_floor_update","task_uid":"{UID}","validation_only":true}}',
            "required-gate\tRun required test tier\t2026-10-03T00:00:19Z\tfirst-activation candidate checker observation: status=passed exit=0 validation_only=true",
            "required-gate\tRun required test tier\t2026-10-03T00:00:20Z\tcargo-package-scope-and-profile-completion: activated candidate checker passed",
            "required-gate\tRun required test tier\t2026-10-03T00:00:16Z\t+ ./scripts/unified-world-code-terminology-scan.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:17Z\tunified-world-code-terminology-scan: OK",
            "required-gate\tRun required test tier\t2026-10-03T00:00:18Z\t+ ./scripts/check-rust-file-size.test.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:19Z\t+ ./scripts/check-rust-file-size.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:20Z\tcheck-rust-file-size: OK",
            "required-gate\tRun required test tier\t2026-10-03T00:00:21Z\t+ bash ./scripts/ci-required-baseline-routing.test.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:22Z\t+ bash ./scripts/ci-required-domain-isolation.test.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:23Z\t+ env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required --verbose",
            "required-gate\tRun required test tier\t2026-10-03T00:00:24Z\t+ python3 ./scripts/pm/first_activation.test.py",
        ]
        return root, base_worktree, base_oid, head_oid, proof_overlay, "\n".join(logs) + "\n"

    def test_full_reconstruction_uses_actual_job_step_commands_and_additive_candidate_test(self):
        class Client:
            def __init__(self, fail_step=False):
                self.fail_step = fail_step

            def rest(self, method, path, *args, **kwargs):
                if "/actions/runs/" in path and "/attempts/" not in path:
                    return {
                        "id": 99, "run_attempt": 1, "event": "workflow_dispatch",
                        "workflow_id": 123456, "path": ".github/workflows/rust.yml",
                        "head_branch": "codex/test", "head_sha": self.head_oid,
                        "repository": {"full_name": "eng-cc/oasis7"},
                        "status": "completed", "conclusion": "success",
                    }
                if "/attempts/1/jobs?" in path:
                    conclusion = "failure" if self.fail_step else "success"
                    return [{
                        "id": 101, "name": "required-gate", "status": "completed",
                        "conclusion": "success", "check_run_url": "https://api.github.com/repos/eng-cc/oasis7/check-runs/102",
                        "steps": [{"name": "Run required test tier", "status": "completed",
                                   "conclusion": conclusion}],
                    }]
                if path == "repos/eng-cc/oasis7/check-runs/102":
                    return {"name": "required-gate", "head_sha": self.head_oid,
                            "status": "completed", "conclusion": "success", "app": {"id": 103}}
                if path == "repos/eng-cc/oasis7":
                    return {"default_branch": "main"}
                raise AssertionError(f"unexpected GitHub API call: {path}")

        with tempfile.TemporaryDirectory() as temp_name:
            temp = pathlib.Path(temp_name)
            root, _base, base_oid, head_oid, bound_overlay, logs = self._fixture(temp)
            client = Client()
            client.head_oid = head_oid
            positive = MODULE.reconstruct_full_required_run(
                root, "eng-cc/oasis7", UID, bound_overlay, 99, 1, client=client,
                log_reader=lambda _run, _attempt: logs,
            )
            self.assertEqual(positive["workflow_run_id"], 99)
            self.assertGreater(positive["obligation_count"], 1)
            self.assertRegex(positive["workflow_logs_sha256"], r"^sha256:[0-9a-f]{64}$")

            echo_decoy = logs.replace(
                "+ ./scripts/lint-skills.sh",
                "+ echo ./scripts/lint-skills.sh",
            )
            with self.assertRaisesRegex(MODULE.OverlayError, "actual command mapped to lint-skills"):
                MODULE.reconstruct_full_required_run(
                    root, "eng-cc/oasis7", UID, bound_overlay, 99, 1, client=client,
                    log_reader=lambda _run, _attempt: echo_decoy,
                )

            cargo_skip = logs.replace(
                "--features test_tier_required --verbose\n",
                "--features test_tier_required --verbose -- --skip offline_server_accepts_client_and_emits_snapshot_and_event\n",
            )
            with self.assertRaisesRegex(MODULE.OverlayError, "direct command for oasis7_required"):
                MODULE.reconstruct_full_required_run(
                    root, "eng-cc/oasis7", UID, bound_overlay, 99, 1, client=client,
                    log_reader=lambda _run, _attempt: cargo_skip,
                )

            missing_test = "\n".join(
                line for line in logs.splitlines() if "first_activation.test.py" not in line
            )
            with self.assertRaisesRegex(MODULE.OverlayError, "candidate-added test command"):
                MODULE.reconstruct_full_required_run(
                    root, "eng-cc/oasis7", UID, bound_overlay, 99, 1, client=client,
                    log_reader=lambda _run, _attempt: missing_test,
                )
            wrong_job = logs.replace("required-gate\tRun required test tier", "other-job\tRun required test tier")
            with self.assertRaisesRegex(MODULE.OverlayError, "required-gate Run required test tier"):
                MODULE.reconstruct_full_required_run(
                    root, "eng-cc/oasis7", UID, bound_overlay, 99, 1, client=client,
                    log_reader=lambda _run, _attempt: wrong_job,
                )
            failed = Client(fail_step=True)
            failed.head_oid = head_oid
            with self.assertRaisesRegex(MODULE.OverlayError, "required-gate has a failed"):
                MODULE.reconstruct_full_required_run(
                    root, "eng-cc/oasis7", UID, bound_overlay, 99, 1, client=failed,
                    log_reader=lambda _run, _attempt: logs,
                )

    def test_checker_leaf_requires_exact_commands_observations_and_candidate_binding(self):
        overlay_value = {"task_uid": UID, "base_oid": BASE, "head_oid": HEAD}
        lines = [
            f"+ python3 /tmp/trusted-check-cargo-package-scope --repo-root {ROOT} --base {BASE} --head {HEAD} --primary-package auto --policy {ROOT}/.pm/cargo-package-scope-policy.json --json",
            '{"status":"rejected","reason":"ambiguous_package_attribution"}',
            "ordinary pull-request trusted checker observation: exit=1 reason=ambiguous_package_attribution status=failed",
            f"+ python3 ./scripts/pm/check-cargo-package-scope --repo-root {ROOT} --base {BASE} --head {HEAD} --first-activation-task-uid {UID} --policy {ROOT}/.pm/cargo-package-scope-policy.json --json",
            f'{{"status":"allowed","mode":"dependency_floor_update","task_uid":"{UID}","validation_only":true}}',
            "ordinary pull-request candidate checker observation: status=passed exit=0 validation_only=true",
        ]
        self.assertTrue(MODULE._valid_checker_json_leaf(lines, overlay_value, candidate_root=ROOT))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace("ambiguous_package_attribution", "checker_internal_error") for line in lines],
            overlay_value, candidate_root=ROOT,
        ))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace(HEAD, "3" * 40) for line in lines], overlay_value, candidate_root=ROOT,
        ))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace(UID, "task_" + "b" * 32) for line in lines], overlay_value,
            candidate_root=ROOT,
        ))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace("--json", "--json --skip unreviewed-option") for line in lines],
            overlay_value, candidate_root=ROOT,
        ))

    def test_command_identity_rejects_echo_and_accepts_runner_wrappers(self):
        self.assertFalse(MODULE._command_is_logged(
            ["+ echo ./scripts/lint-skills.sh"], "./scripts/lint-skills.sh",
        ))
        self.assertFalse(MODULE._path_is_logged(
            ["+ echo ./scripts/lint-skills.sh"], "./scripts/lint-skills.sh",
        ))
        self.assertTrue(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required --verbose"],
            "cargo test -p oasis7 --tests --features test_tier_required",
        ))
        self.assertFalse(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required --verbose -- --skip offline_test"],
            "cargo test -p oasis7 --tests --features test_tier_required",
        ))
        self.assertTrue(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo clippy --verbose -p oasis7 --lib -- -D warnings"],
            "cargo clippy -p oasis7 --lib -- -D warnings",
        ))
        self.assertFalse(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo clippy --verbose -p oasis7 --lib -- -D warnings --allow=all"],
            "cargo clippy -p oasis7 --lib -- -D warnings",
        ))
        self.assertTrue(MODULE._path_is_logged(
            ["+ bash ./scripts/check-script-executable-bits.sh"],
            "scripts/check-script-executable-bits.sh",
        ))


if __name__ == "__main__":
    unittest.main()
