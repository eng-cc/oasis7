#!/usr/bin/env python3
"""C3 RED contracts for package-profile provenance envelopes."""

from __future__ import annotations

import importlib.util
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import zipfile


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/rust.yml"
CI_READY = ROOT / "scripts/pm/ci-ready-receipt.py"
CI_READY_TEST = ROOT / "scripts/pm/ci-ready-receipt.test.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CargoPackageProfileEnvelopeRED(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.receipt_tests = load_module(CI_READY_TEST, "ci_ready_receipt_c3_envelope")

    def _envelope(self, **overrides: object) -> dict[str, object]:
        envelope: dict[str, object] = {
            "schema": "oasis7-cargo-package-profile-envelope/v1",
            "repository": "eng-cc/oasis7",
            "task_uid": "task_12345678901234567890123456789012",
            "task_issue_number": 1,
            "pr_number": 7,
            "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
            "workflow_sha": "b" * 40,
            "run_id": 9,
            "run_attempt": 1,
            "check_name": "required-gate",
            "check_app_id": 42,
            "check_run_id": 9,
            "integration_base": "b" * 40,
            "source_head": "a" * 40,
            "tested_tree": "t" * 40,
            "plan_digest": "sha256:" + "1" * 64,
            "results_digest": "sha256:" + "2" * 64,
            "receipt_digest": "sha256:" + "3" * 64,
        }
        envelope.update(overrides)
        return envelope

    def test_workflow_and_live_receipt_consume_one_profile_provenance_envelope(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        profile_start = required.index('profile_output="${GITHUB_WORKSPACE}/output/cargo-package-profile"')
        profile_block = required[profile_start:]
        for field in (
            "cargo-package-profile-envelope",
            "task_uid",
            "pr_number",
            "repository",
            "workflow_ref",
            "workflow_sha",
            "run_id",
            "run_attempt",
            "check_app_id",
            "check_run_id",
            "integration_base",
            "source_head",
            "tested_tree",
            "plan_digest",
            "results_digest",
            "receipt_digest",
        ):
            with self.subTest(field=field):
                self.assertIn(field, profile_block)

        receipt_source = CI_READY.read_text(encoding="utf-8")
        for field in (
            "cargo_package_profile",
            "plan_digest",
            "results_digest",
            "receipt_digest",
        ):
            with self.subTest(ci_ready_field=field):
                self.assertIn(field, receipt_source)

    def test_envelope_lookup_binds_repository_token_only_to_builder_process(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        profile_step = required.split(
            "      - name: Run required test tier\n", 1
        )[1].split("\n      - name:", 1)[0]
        env_block = profile_step.split("        env:\n", 1)[1].split(
            "\n        run:", 1
        )[0]
        self.assertNotIn(
            "GH_TOKEN:",
            env_block,
            "the repository token must not be exposed to the entire required-test step",
        )
        self.assertIn(
            'GH_TOKEN="${{ github.token }}" python3 -I - "${OASIS7_CARGO_PROFILE_PLAN}"',
            profile_step,
            "the envelope lookup must receive the repository-scoped token process-locally",
        )
        self.assertIn("unset GH_TOKEN GITHUB_TOKEN", profile_step)
        for step_name in ("Freeze internal same-attempt dispatch and exact Git objects", "Execute only the frozen selected worker"):
            step = workflow.split(f"        name: {step_name}\n", 1)[1] if step_name.startswith("Freeze") else workflow.split(f"      - name: {step_name}\n", 1)[1]
            step = step.split("\n      - ", 1)[0]
            self.assertNotIn("GH_TOKEN:", step)
            self.assertNotIn("GITHUB_TOKEN:", step)

    def test_ci_ready_rejects_detached_cross_run_or_digest_mismatch_profile_receipt(self) -> None:
        case = self.receipt_tests.ReceiptTest("test_success")
        module=self.receipt_tests.M
        original_api=case.api
        inventories=[]
        @contextlib.contextmanager
        def api_with_changed_inventory(*args,**kwargs):
            with original_api(*args,**kwargs):
                original=module.gh
                def read(*argv):
                    if argv[-1]=='repos/eng-cc/oasis7/pulls/7/files?per_page=100':
                        self.assertEqual(argv,('api','--paginate','--slurp',argv[-1]))
                        inventories.append(argv)
                        return [[{'filename':'scripts/pm/cargo-package-profile-envelope-c3-red.test.py','status':'modified'}]]
                    return original(*argv)
                with patch.object(module,'gh',side_effect=read):yield
        case.api=api_with_changed_inventory
        mutations = {
            "run_id": 999,  # cross-run evidence
            "repository": "other/repo",
            "plan_digest": "sha256:" + "9" * 64,  # digest mismatch
        }
        for field, bad_value in mutations.items():
            def mutate(receipt: dict[str, object], field=field, bad_value=bad_value) -> None:
                envelope = self._envelope()
                envelope[field] = bad_value
                receipt["cargo_package_profile"] = envelope

            with self.subTest(field=field), self.assertRaisesRegex(
                ValueError, "^ci-ready-receipt: package profile evidence is detached from a trusted integration run$"
            ):
                case.invoke_verify(mutate)
        self.assertEqual(len(inventories),len(mutations))

    def test_actual_profile_artifacts_reject_exact_cross_run_repository_and_plan_digest(self) -> None:
        module=self.receipt_tests.M
        proof={'workflow_run_id':9,'workflow_sha':'b'*40,
            'workflow_ref':'eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main',
            'run_attempt':1,'base_oid':'b'*40,'head_oid':'a'*40,'tested_tree_oid':'t'*40}
        check={'name':'required-gate','app':{'id':42},'id':9}
        payloads={'plan':b'actual profile plan','results':b'actual profile results','receipt':b'actual profile receipt'}
        envelope=self._envelope(**{key+'_digest':'sha256:'+hashlib.sha256(value).hexdigest() for key,value in payloads.items()})
        for field,bad,reason in ((None,None,None),('run_id',999,'identity mismatch: run_id'),
                ('repository','other/repo','identity mismatch: repository'),
                ('plan_digest','sha256:'+'9'*64,'digest mismatch: plan')):
            value=dict(envelope)
            if field:value[field]=bad
            contents={**payloads,'envelope':json.dumps(value).encode()}
            artifacts=[];archives={}
            for ident,(key,(name,member)) in enumerate(module.PROFILE_ARTIFACTS.items(),1):
                artifacts.append({'id':ident,'name':name,'expired':False,'workflow_run':{'id':9}})
                buffer=io.BytesIO()
                with zipfile.ZipFile(buffer,'w') as archive:archive.writestr(member,contents[key])
                archives[ident]=buffer.getvalue()
            calls=[]
            def read(*args):
                self.assertEqual(args,('api','repos/eng-cc/oasis7/actions/runs/9/artifacts?per_page=100&page=1'))
                calls.append(args);return {'artifacts':artifacts}
            with self.subTest(field=field),patch.object(module,'gh',side_effect=read),patch.object(module,'artifact_bytes',side_effect=lambda repository,ident:archives[ident]):
                if field:
                    with self.assertRaisesRegex(SystemExit,'^ci-ready-receipt: package profile '+reason+'$'):
                        module.cargo_package_profile_for_run('eng-cc/oasis7',check,proof,{},task_uid=envelope['task_uid'],task_issue_number=1,pr_number=7)
                else:
                    self.assertEqual(module.cargo_package_profile_for_run('eng-cc/oasis7',check,proof,{},task_uid=envelope['task_uid'],task_issue_number=1,pr_number=7),envelope)
            self.assertEqual(len(calls),1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
