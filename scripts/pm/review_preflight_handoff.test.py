#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Callable
import unittest


HERE = Path(__file__).resolve().parent
BATCH_SCRIPT = HERE / "review-batch-epoch.py"
HANDOFF_SCRIPT = HERE / "review_preflight_handoff.py"
PROJECTION_SCRIPT = HERE / "workflow-impact-projection.py"
PROJECT_ROOT = HERE.parent.parent
TASK = "task_" + "1" * 32
TASK_ISSUE = 4137
DISPATCH_COMMENT_ID = 3934017999
DISPATCH_AUTHOR = "repo-admin"
ROLE = "qa_engineer"
SLICE = "11111111-1111-4111-8111-111111111111"
HEAD = "a" * 40
SCOPE_OID = "c" * 40
REPOSITORY = "eng-cc/oasis7"
ACTIVATION = "message-assigned"
CONTEXT_DELIVERY = "minimal-task-packet"
MODEL_REASONING = "inherited/unverified"
RUNTIME_REASON = (
    "message-assigned fallback; adapter inactive on this surface; "
    "actual runtime/model/reasoning unverified"
)
DISPATCH_MARKER = "<!-- oasis7-review-dispatch/v1 -->"


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def load_handoff_module():
    spec = importlib.util.spec_from_file_location("review_preflight_handoff_under_test", HANDOFF_SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_projection_module():
    spec = importlib.util.spec_from_file_location("workflow_impact_projection_under_test", PROJECTION_SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HANDOFF = load_handoff_module()
IMPACT_PROJECTION = load_projection_module()


class ReviewPreflightHandoffTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.task_root = self.root / ".pm" / "scratch" / TASK
        mapping_root = self.root / ".pm" / "github-project-sync"
        mapping_root.mkdir(parents=True, exist_ok=True)
        (mapping_root / "tasks.json").write_text(json.dumps({
            "project": {"repo": REPOSITORY},
            "tasks": {TASK: {"issue_number": TASK_ISSUE}},
        }) + "\n", encoding="utf-8")
        self.gh_log = self.root / "gh.log"
        self.gh_data = self.root / "gh-fixture.json"
        self.gh_bin = self.root / "bin"
        self.gh_bin.mkdir()
        self.fake_gh = self.gh_bin / "gh"
        self.fake_gh.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "args = sys.argv[1:]\n"
            "open(os.environ['GH_LOG'], 'a').write(' '.join(args) + '\\n')\n"
            "if not args or args[0] != 'api': raise SystemExit('unexpected gh call: ' + ' '.join(args))\n"
            "endpoint = next((arg for arg in args[1:] if arg.startswith('repos/')), None)\n"
            "if endpoint is None: raise SystemExit('missing endpoint: ' + ' '.join(args))\n"
            "with open(os.environ['GH_FIXTURE'], encoding='utf-8') as handle: data = json.load(handle)\n"
            "method = next((args[i + 1] for i, arg in enumerate(args[:-1]) if arg == '--method'), 'GET')\n"
            "if method == 'POST' and endpoint == f\"repos/{data['repository']}/issues/{data['issue_number']}/comments\":\n"
            "    body = next((args[i + 1][5:] for i, arg in enumerate(args[:-1]) if arg in ('--field', '-f') and args[i + 1].startswith('body=')), None)\n"
            "    if body is None: raise SystemExit('missing POST body')\n"
            "    comment = {'id': data['next_comment_id'], 'body': body, 'issue_url': f\"https://api.github.com/repos/{data['repository']}/issues/{data['issue_number']}\", 'html_url': f\"https://github.com/{data['repository']}/issues/{data['issue_number']}#issuecomment-{data['next_comment_id']}\", 'user': {'login': data['post_author']}, 'created_at': '2026-09-29T00:00:00Z'}\n"
            "    if not data['comment_pages']: data['comment_pages'] = [[]]\n"
            "    data['comment_pages'][-1].append(comment)\n"
            "    with open(os.environ['GH_FIXTURE'], 'w', encoding='utf-8') as handle: json.dump(data, handle, ensure_ascii=False, sort_keys=True)\n"
            "    print(json.dumps(comment))\n"
            "elif endpoint == f\"repos/{data['repository']}/issues/{data['issue_number']}\":\n"
            "    print(json.dumps(data['issue']))\n"
            "elif endpoint.startswith(f\"repos/{data['repository']}/issues/{data['issue_number']}/comments\"):\n"
            "    pages = data['comment_pages']\n"
            "    print(json.dumps(pages if '--paginate' in args else (pages[0] if pages else [])))\n"
            "elif endpoint.startswith(f\"repos/{data['repository']}/issues/comments/\"):\n"
            "    comment_id = int(endpoint.rsplit('/', 1)[1])\n"
            "    matches = [comment for page in data['comment_pages'] for comment in page if comment.get('id') == comment_id]\n"
            "    if len(matches) != 1: raise SystemExit('comment fixture is not unique')\n"
            "    print(json.dumps(matches[0]))\n"
            "elif endpoint.startswith(f\"repos/{data['repository']}/collaborators/\") and endpoint.endswith('/permission'):\n"
            "    print(json.dumps({'permission': data['permission']}))\n"
            "else: raise SystemExit('unexpected gh endpoint: ' + endpoint)\n",
            encoding="utf-8",
        )
        self.fake_gh.chmod(0o755)
        old_path = os.environ.get("PATH")
        os.environ["PATH"] = f"{self.gh_bin}:{old_path or ''}"
        self.addCleanup(self._restore_path, old_path)
        old_log = os.environ.get("GH_LOG")
        old_fixture = os.environ.get("GH_FIXTURE")
        os.environ["GH_LOG"] = str(self.gh_log)
        os.environ["GH_FIXTURE"] = str(self.gh_data)
        self.addCleanup(self._restore_env, "GH_LOG", old_log)
        self.addCleanup(self._restore_env, "GH_FIXTURE", old_fixture)

    @staticmethod
    def _restore_path(value: str | None) -> None:
        if value is None:
            os.environ.pop("PATH", None)
        else:
            os.environ["PATH"] = value

    @staticmethod
    def _restore_env(key: str, value: str | None) -> None:
        if value is None:
            os.environ.pop(key, None)
        else:
            os.environ[key] = value

    def tearDown(self) -> None:
        self.temp.cleanup()

    def command_json(self, *args: str) -> dict[str, object]:
        result = subprocess.run(
            [sys.executable, str(BATCH_SCRIPT), "--root", str(self.root), *args],
            text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        return json.loads(result.stdout)

    def run_dispatch(self, fixture: dict[str, object], *, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            [sys.executable, str(BATCH_SCRIPT), "--root", str(self.root), "dispatch",
             "--plan", str(fixture["plan_path"])],
            text=True, capture_output=True,
        )
        self.assertEqual(0 if ok else 2, result.returncode, result.stdout + result.stderr)
        return result

    def run_handoff(self, fixture: dict[str, object], *, ok: bool = True,
                    comment_id: int = DISPATCH_COMMENT_ID) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            [sys.executable, str(BATCH_SCRIPT), "--root", str(self.root), "handoff",
             "--plan", str(fixture["plan_path"]), "--dispatch-comment-id", str(comment_id)],
            text=True, capture_output=True,
        )
        self.assertEqual(0 if ok else 2, result.returncode, result.stdout + result.stderr)
        return result

    def write_live_issue(self, body: str | None, *, author: str = DISPATCH_AUTHOR,
                         permission: str = "admin", issue_task_uid: str = TASK,
                         issue_pr_number: int = 1, pages: list[list[dict[str, object]]] | None = None) -> None:
        comment = {
            "id": DISPATCH_COMMENT_ID,
            "body": body,
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/{TASK_ISSUE}",
            "html_url": f"https://github.com/{REPOSITORY}/issues/{TASK_ISSUE}#issuecomment-{DISPATCH_COMMENT_ID}",
            "user": {"login": author},
            "created_at": "2026-09-29T00:00:00Z",
        }
        if pages is None:
            prior = [{
                "id": DISPATCH_COMMENT_ID - 1, "body": "unrelated prior task comment",
                "issue_url": comment["issue_url"], "user": {"login": "someone"},
                "created_at": "2026-09-28T00:00:00Z",
            }]
            pages = [prior, [comment] if body is not None else []]
        issue_body = (
            f"<!-- oasis7-pm-task -->\ntask_uid: {issue_task_uid}\n"
            f"- pr_url: `https://github.com/{REPOSITORY}/pull/{issue_pr_number}`\n"
            f"- pr_number: `{issue_pr_number}`\n"
        )
        self.gh_data.write_text(json.dumps({
            "repository": REPOSITORY,
            "issue_number": TASK_ISSUE,
            "issue": {
                "number": TASK_ISSUE,
                "html_url": f"https://github.com/{REPOSITORY}/issues/{TASK_ISSUE}",
                "body": issue_body,
            },
            "comment_pages": pages,
            "permission": permission,
            "next_comment_id": DISPATCH_COMMENT_ID,
            "post_author": author,
        }, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

    def dispatch_payload(self, plan: dict[str, object], plan_path: Path,
                         batch_path: Path) -> dict[str, object]:
        packet_refs = plan["packet_refs"]
        assert isinstance(packet_refs, list)
        rows: list[dict[str, object]] = []
        for ref in packet_refs:
            assert isinstance(ref, dict)
            packet_path = self.root / str(ref["packet_ref"])
            packet = json.loads(packet_path.read_text(encoding="utf-8"))
            rows.append({
                "role": ref["role"], "slice_id": ref["slice_id"],
                "packet_path": str(ref["packet_ref"]), "packet_digest": packet["packet_digest"],
            })
        rows.sort(key=lambda row: (str(row["role"]).encode(), str(row["slice_id"]).encode()))
        batch = json.loads(batch_path.read_text(encoding="utf-8"))
        source_identity = plan["source_review_identity"]
        assert isinstance(source_identity, dict)
        return {
            "schema": "oasis7-review-dispatch/v1",
            "repository": REPOSITORY,
            "task_uid": plan["task_uid"],
            "issue_number": TASK_ISSUE,
            "pr_number": source_identity["pr_number"],
            "frozen_head": plan["frozen_head"],
            "epoch": plan["epoch"],
            "plan_path": plan_path.relative_to(self.root).as_posix(),
            "plan_sha256": hashlib.sha256(plan_path.read_bytes()).hexdigest(),
            "batch_path": batch_path.relative_to(self.root).as_posix(),
            "batch_sha256": hashlib.sha256(batch_path.read_bytes()).hexdigest(),
            "rows": rows,
        }

    @staticmethod
    def dispatch_body(payload: dict[str, object]) -> str:
        return f"{DISPATCH_MARKER}\n```json\n{canonical(payload).decode('utf-8')}\n```"

    def make_fixture(self, *, create_handoff: bool = True) -> dict[str, object]:
        expected_slices = [{"role": ROLE, "slice_id": SLICE}]
        impact_projection = IMPACT_PROJECTION.build_projection(
            PROJECT_ROOT,
            {
                "task_uid": TASK,
                "source_head_oid": HEAD,
                "scope_base_oid": SCOPE_OID,
                "changed_paths": ["scripts/pm/review_preflight_handoff.test.py"],
                "change_class": "unknown",
                "manual_roles": [ROLE],
                "domain_role": None,
                "test_profile": "required",
                "declared_tests": ["review-preflight-handoff"],
                "consumed_contracts": [],
                "public_semantics": [],
                "affected_consumers": [],
                "closure_status": {
                    "status": "unknown",
                    "reason": "fixture impact remains open",
                    "evidence": [],
                },
                "verification_affected": True,
            },
        )
        source_identity = {
            "task_uid": TASK,
            "bootstrap_epoch": 1,
            "repository": REPOSITORY,
            "pr_number": 1,
            "source_head_oid": HEAD,
            "source_scope_oid": SCOPE_OID,
            "changed_paths_digest": impact_projection["changed_paths_digest"].removeprefix("sha256:"),
            "ordered_role_ids": impact_projection["ordered_role_ids"],
            "role_contract_digest": "d" * 64,
            "review_policy_digest": "e" * 64,
            "input_contract_digest": impact_projection["projection_digest"].removeprefix("sha256:"),
        }
        source_digest = digest(source_identity)
        batch_path = self.task_root / "review-batches" / "batch.json"
        batch_path.parent.mkdir(parents=True, exist_ok=True)
        batch = self.command_json(
            "create", "--task-uid", TASK, "--head", HEAD,
            "--evidence-digest", source_digest, "--slice", f"{ROLE}={SLICE}",
            "--out", str(batch_path),
        )
        epoch = str(batch["epoch"])
        preflight_dir = self.task_root / "review-plans" / "preflight"
        preflight = self.command_json(
            "preflight", "--batch", str(batch_path), "--out-dir", str(preflight_dir)
        )
        return_path = preflight_dir / f"{SLICE}.json"
        returned = json.loads(return_path.read_text(encoding="utf-8"))
        returned.update({
            "status": "completed",
            "activation": ACTIVATION,
            "context_delivery": CONTEXT_DELIVERY,
            "actual_runtime": f"{MODEL_REASONING}: {RUNTIME_REASON}",
            "scope_verdict": "approved",
            "risk_verdict": "approved",
            "disposition": "no_findings",
            "findings": [],
            "residual_risk": "fixture risk",
        })

        packet_path = self.task_root / "slice-packets" / f"{SLICE}.json"
        packet_path.parent.mkdir(parents=True, exist_ok=True)
        packet: dict[str, object] = {
            "schema": "oasis7-subagent-task-packet/v1",
            "identity": {"task_uid": TASK, "head": HEAD, "base_sha": SCOPE_OID},
            "slice": {
                "slice_id": SLICE,
                "role": ROLE,
                "role_activation": "message_assigned_adapter_inactive",
                "context_delivery_mode": "minimal_head_bound_task_packet",
                "actual_dispatched_model_reasoning": MODEL_REASONING,
                "actual_runtime_evidence_reason": RUNTIME_REASON,
            },
        }
        packet["packet_digest"] = digest(packet)
        packet_path.write_text(json.dumps(packet, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

        applicability_identity = {
            key: source_identity[key] for key in (
                "changed_paths_digest", "input_contract_digest", "ordered_role_ids",
                "role_contract_digest", "review_policy_digest",
            )
        }
        plan_path = self.task_root / "review-plans" / "plan.json"
        plan_path.parent.mkdir(parents=True, exist_ok=True)
        plan = {
            "schema": "oasis7-review-plan/v2",
            "task_uid": TASK,
            "frozen_head": HEAD,
            "comparison_ref": "refs/heads/main",
            "comparison_oid": SCOPE_OID,
            "source_scope_oid": SCOPE_OID,
            "source_review_identity": source_identity,
            "source_review_digest": source_digest,
            "relevant_evidence_digest": source_digest,
            "professional_review_applicability": {
                "identity": applicability_identity,
                "identity_digest": digest(applicability_identity),
                "verified": True,
            },
            "impact_projection": impact_projection,
            "impact_projection_schema": impact_projection["schema"],
            "impact_projection_digest": impact_projection["projection_digest"],
            "impact_projection_test_profile": impact_projection["test_profile"],
            "impact_projection_declared_tests": impact_projection["declared_tests"],
            "impact_projection_planner_digest": impact_projection["planner_digest"],
            "epoch": epoch,
            "batch_path": str(batch_path),
            "collection_path": str(batch_path.with_name("batch.collection.json")),
            "roles": [ROLE],
            "expected_slices": expected_slices,
            "packet_refs": [{
                "role": ROLE, "slice_id": SLICE,
                "packet_ref": packet_path.relative_to(self.root).as_posix(),
            }],
            "preflight": {"status": "incomplete", "ledger_path": preflight["ledger_path"]},
        }
        plan_path.write_text(json.dumps(plan, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        dispatch = self.dispatch_payload(plan, plan_path, batch_path)
        dispatch_comment_body = self.dispatch_body(dispatch)
        self.write_live_issue(None)
        fixture = {
            "batch": batch,
            "batch_path": batch_path,
            "collection_path": Path(str(plan["collection_path"])),
            "dispatch_body": dispatch_comment_body,
            "dispatch_payload": dispatch,
            "epoch": epoch,
            "handoff_path": self.task_root / "review-handoffs" / f"{epoch}.json",
            "plan": plan,
            "plan_path": plan_path,
            "packet_path": packet_path,
            "return_path": return_path,
            "ledger_path": Path(str(plan["preflight"]["ledger_path"])),
            "source_digest": source_digest,
        }
        dispatch_result = json.loads(self.run_dispatch(fixture).stdout)
        self.assertEqual(TASK, dispatch_result["task_uid"])
        self.assertEqual(TASK_ISSUE, dispatch_result["issue_number"])
        self.assertEqual(1, dispatch_result["pr_number"])
        self.assertEqual(HEAD, dispatch_result["head"])
        self.assertEqual(epoch, dispatch_result["epoch"])
        self.assertEqual(DISPATCH_COMMENT_ID, dispatch_result["dispatch_comment_id"])
        self.assertEqual(DISPATCH_AUTHOR, dispatch_result["author"])
        self.assertEqual(f"https://api.github.com/repos/{REPOSITORY}/issues/{TASK_ISSUE}",
                         dispatch_result["issue_url"])
        row_matches = [row for row in dispatch_result["rows"]
                       if row.get("role") == ROLE and row.get("slice_id") == SLICE]
        self.assertEqual(1, len(row_matches), dispatch_result)
        self.assertEqual(packet["packet_digest"], row_matches[0]["packet_digest"])
        returned["admitted_packet_digest"] = row_matches[0]["packet_digest"]
        return_path.write_text(json.dumps(returned, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        live = json.loads(self.gh_data.read_text(encoding="utf-8"))
        posted = [comment for page in live["comment_pages"] for comment in page
                  if comment.get("id") == DISPATCH_COMMENT_ID]
        self.assertEqual(1, len(posted))
        self.assertEqual(dispatch_comment_body, posted[0]["body"])
        self.assertEqual(hashlib.sha256(dispatch_comment_body.encode("utf-8")).hexdigest(),
                         dispatch_result["body_digest"])
        fixture["dispatch_body"] = dispatch_comment_body
        fixture["dispatch_result"] = dispatch_result
        fixture["dispatch_comment_id"] = DISPATCH_COMMENT_ID
        if create_handoff:
            self.run_handoff(fixture, comment_id=DISPATCH_COMMENT_ID)
        return fixture

    def rebind_return_and_handoff(self, fixture: dict[str, object], field: str, value: str) -> None:
        return_path = Path(str(fixture["return_path"]))
        returned = json.loads(return_path.read_text(encoding="utf-8"))
        returned[field] = value
        raw = json.dumps(returned, ensure_ascii=False, sort_keys=True).encode() + b"\n"
        return_path.write_bytes(raw)
        handoff_path = Path(str(fixture["handoff_path"]))
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        row = handoff["rows"][0]
        row["return_sha256"] = hashlib.sha256(raw).hexdigest()
        row["findings_digest"] = digest(returned["findings"])
        handoff["handoff_digest"] = digest({key: item for key, item in handoff.items() if key != "handoff_digest"})
        handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

    def rebind_return_packet_digest(self, fixture: dict[str, object], value: str | None) -> None:
        return_path = Path(str(fixture["return_path"]))
        returned = json.loads(return_path.read_text(encoding="utf-8"))
        if value is None:
            returned.pop("admitted_packet_digest", None)
        else:
            returned["admitted_packet_digest"] = value
        raw = json.dumps(returned, ensure_ascii=False, sort_keys=True).encode() + b"\n"
        return_path.write_bytes(raw)
        handoff_path = Path(str(fixture["handoff_path"]))
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        row = handoff["rows"][0]
        row["return_sha256"] = hashlib.sha256(raw).hexdigest()
        row["findings_digest"] = digest(returned["findings"])
        handoff["handoff_digest"] = digest({key: item for key, item in handoff.items() if key != "handoff_digest"})
        handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

    def rebind_handoff_dispatch_body(self, fixture: dict[str, object], body: str) -> None:
        handoff_path = Path(str(fixture["handoff_path"]))
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        handoff["dispatch_evidence"]["body_digest"] = hashlib.sha256(body.encode("utf-8")).hexdigest()
        handoff["handoff_digest"] = digest({key: item for key, item in handoff.items() if key != "handoff_digest"})
        handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

    def test_accepts_return_metadata_bound_to_slice_packet(self) -> None:
        fixture = self.make_fixture()
        validated = HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))
        returned = validated["returns"][(ROLE, SLICE)][2]
        handoff = validated["handoff"]
        self.assertEqual(ACTIVATION, returned["activation"])
        self.assertEqual(CONTEXT_DELIVERY, returned["context_delivery"])
        self.assertEqual(f"{MODEL_REASONING}: {RUNTIME_REASON}", returned["actual_runtime"])
        self.assertEqual(fixture["dispatch_result"]["rows"][0]["packet_digest"], returned["admitted_packet_digest"])
        self.assertEqual("oasis7-review-return-handoff/v2", handoff["schema"])
        self.assertEqual({"issue_number", "issue_url", "comment_id", "author", "body_digest"},
                         set(handoff["dispatch_evidence"]))
        self.assertEqual(TASK_ISSUE, handoff["dispatch_evidence"]["issue_number"])
        self.assertEqual(f"https://api.github.com/repos/{REPOSITORY}/issues/{TASK_ISSUE}",
                         handoff["dispatch_evidence"]["issue_url"])
        self.assertEqual(DISPATCH_COMMENT_ID, handoff["dispatch_evidence"]["comment_id"])
        self.assertEqual(DISPATCH_AUTHOR, handoff["dispatch_evidence"]["author"])
        self.assertEqual(hashlib.sha256(str(fixture["dispatch_body"]).encode("utf-8")).hexdigest(),
                         handoff["dispatch_evidence"]["body_digest"])
        comments_reads = [line for line in self.gh_log.read_text(encoding="utf-8").splitlines()
                          if f"issues/{TASK_ISSUE}/comments?per_page=100" in line]
        self.assertTrue(any("--paginate" in line and "--slurp" in line for line in comments_reads),
                        self.gh_log.read_text(encoding="utf-8"))
        row = handoff["rows"][0]
        self.assertEqual({"role", "slice_id", "packet_path", "packet_digest", "artifact_path",
                          "return_sha256", "findings_digest"}, set(row))
        self.assertEqual(fixture["dispatch_result"]["rows"][0]["packet_path"], row["packet_path"])
        self.assertEqual(fixture["dispatch_result"]["rows"][0]["packet_digest"], row["packet_digest"])

    def test_v2_plan_without_impact_projection_is_rejected_without_side_effects(self) -> None:
        fixture = self.make_fixture(create_handoff=False)
        plan = dict(fixture["plan"])
        valid_plan_raw = canonical(plan) + b"\n"
        validated, _, _ = HANDOFF.validate_plan(plan, valid_plan_raw)
        self.assertEqual(plan, validated)

        plan.pop("impact_projection")
        missing_projection_raw = canonical(plan) + b"\n"
        plan_path = Path(str(fixture["plan_path"]))
        plan_path.write_bytes(missing_projection_raw)
        ledger_path = Path(str(fixture["ledger_path"]))
        collection_path = Path(str(fixture["collection_path"]))
        handoff_path = Path(str(fixture["handoff_path"]))
        ledger_before = ledger_path.read_bytes()
        self.assertFalse(collection_path.exists())
        self.assertFalse(handoff_path.exists())

        rejection = None
        try:
            HANDOFF.validate_plan(plan, missing_projection_raw)
        except HANDOFF.ContractError as error:
            rejection = error

        self.assertEqual(ledger_before, ledger_path.read_bytes())
        self.assertFalse(collection_path.exists())
        self.assertFalse(handoff_path.exists())
        self.assertIsNotNone(rejection, "v2 plan without impact_projection was accepted")
        self.assertRegex(str(rejection), "impact projection")

    def assert_dispatch_rejects_projection_mutation_without_side_effects(
        self, fixture: dict[str, object], *, expected_error: str,
    ) -> None:
        plan_path = Path(str(fixture["plan_path"]))
        ledger_path = Path(str(fixture["ledger_path"]))
        collection_path = Path(str(fixture["collection_path"]))
        handoff_path = Path(str(fixture["handoff_path"]))
        # The fixture's normal setup publishes the original valid plan. Remove that
        # marker so a mutated plan reaches projection validation before publication.
        self.write_live_issue(None)
        gh_log_before = self.gh_log.read_bytes()
        gh_fixture_before = self.gh_data.read_bytes()
        ledger_before = ledger_path.read_bytes()
        self.assertFalse(collection_path.exists())
        self.assertFalse(handoff_path.exists())

        failure = subprocess.run(
            [sys.executable, str(BATCH_SCRIPT), "--root", str(self.root), "dispatch",
             "--plan", str(plan_path)],
            text=True, capture_output=True,
        )

        self.assertEqual(gh_log_before, self.gh_log.read_bytes(), "invalid projection reached GitHub")
        self.assertEqual(gh_fixture_before, self.gh_data.read_bytes(), "invalid projection changed live fixture")
        self.assertEqual(ledger_before, ledger_path.read_bytes(), "invalid projection changed preflight ledger")
        self.assertFalse(collection_path.exists(), "invalid projection created a collection receipt")
        self.assertFalse(handoff_path.exists(), "invalid projection created a handoff")
        self.assertEqual(2, failure.returncode, failure.stdout + failure.stderr)
        self.assertRegex((failure.stderr + failure.stdout).lower(), expected_error)

    @staticmethod
    def rewrite_projection_plan(
        fixture: dict[str, object], mutate: Callable[[dict[str, object]], None], *,
        rebind_projection_digest: bool = False,
    ) -> dict[str, object]:
        plan_path = Path(str(fixture["plan_path"]))
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
        projection = plan["impact_projection"]
        assert isinstance(projection, dict)
        mutate(projection)
        if rebind_projection_digest:
            projection["projection_digest"] = IMPACT_PROJECTION.canonical_digest(
                {key: value for key, value in projection.items() if key != "projection_digest"}
            )
            plan["impact_projection_digest"] = projection["projection_digest"]
        plan_path.write_bytes(canonical(plan) + b"\n")
        return plan

    def test_valid_v2_impact_projection_is_bound_to_source_review_identity(self) -> None:
        fixture = self.make_fixture(create_handoff=False)
        plan_path = Path(str(fixture["plan_path"]))
        validated, source_identity, *_ = HANDOFF.validate_plan_inputs(self.root, plan_path)
        projection = validated["impact_projection"]
        assert isinstance(projection, dict)
        projection_path = self.root / "impact-projection.json"
        projection_path.write_bytes(canonical(projection) + b"\n")
        verified = IMPACT_PROJECTION.load_verified_projection(
            projection_path,
            expected={
                "task_uid": TASK,
                "source_head_oid": HEAD,
                "scope_base_oid": SCOPE_OID,
                "changed_paths_digest": projection["changed_paths_digest"],
                "ordered_role_ids": source_identity["ordered_role_ids"],
            },
        )
        self.assertEqual(projection, verified)
        self.assertEqual(
            "sha256:" + str(source_identity["input_contract_digest"]),
            projection["projection_digest"],
        )
        payload = HANDOFF.dispatch_payload_for_plan(self.root, plan_path)
        self.assertEqual(TASK, payload["task_uid"])

    def test_dispatch_rejects_stale_impact_projection_digest_after_closure_mutation(self) -> None:
        fixture = self.make_fixture(create_handoff=False)

        def forge_complete_closure(projection: dict[str, object]) -> None:
            projection["closure_status"] = {
                "status": "complete", "reason": "forged evidence closure", "evidence": [],
            }

        self.rewrite_projection_plan(fixture, forge_complete_closure)
        self.assert_dispatch_rejects_projection_mutation_without_side_effects(
            fixture, expected_error=r"projection|digest|closure|identity",
        )

    def test_dispatch_rejects_stale_impact_projection_digest_after_ci_scope_mutation(self) -> None:
        fixture = self.make_fixture(create_handoff=False)

        def change_ci_scope(projection: dict[str, object]) -> None:
            projection["ci_scope"] = "minimal" if projection["ci_scope"] != "minimal" else "full"

        self.rewrite_projection_plan(fixture, change_ci_scope)
        self.assert_dispatch_rejects_projection_mutation_without_side_effects(
            fixture, expected_error=r"projection|digest|scope|identity",
        )

    def test_dispatch_rejects_rehashed_projection_with_changed_paths_binding(self) -> None:
        fixture = self.make_fixture(create_handoff=False)

        def change_paths_binding(projection: dict[str, object]) -> None:
            paths = ["scripts/pm/review_preflight_handoff.py"]
            projection["changed_paths"] = paths
            projection["changed_paths_digest"] = IMPACT_PROJECTION.canonical_digest(paths)

        self.rewrite_projection_plan(fixture, change_paths_binding, rebind_projection_digest=True)
        self.assert_dispatch_rejects_projection_mutation_without_side_effects(
            fixture, expected_error=r"projection|digest|path|identity|contract",
        )

    def test_dispatch_rejects_rehashed_projection_with_changed_roles_binding(self) -> None:
        fixture = self.make_fixture(create_handoff=False)

        def change_role_binding(projection: dict[str, object]) -> None:
            projection["review_roles"] = ["runtime_engineer"]
            projection["ordered_role_ids"] = ["runtime_engineer"]

        self.rewrite_projection_plan(fixture, change_role_binding, rebind_projection_digest=True)
        self.assert_dispatch_rejects_projection_mutation_without_side_effects(
            fixture, expected_error=r"projection|digest|role|identity|contract",
        )

    def test_rejects_missing_admitted_packet_digest_after_return_and_handoff_rehash(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_packet_digest(fixture, None)
        with self.assertRaisesRegex(HANDOFF.ContractError, "admitted.*packet|packet.*digest|packet metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_wrong_admitted_packet_digest_after_return_and_handoff_rehash(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_packet_digest(fixture, "9" * 64)
        with self.assertRaisesRegex(HANDOFF.ContractError, "admitted.*packet|packet.*digest|packet metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_dispatch_rejects_self_consistent_packet_substitution_without_side_effects(self) -> None:
        fixture = self.make_fixture(create_handoff=False)
        packet_path = Path(str(fixture["packet_path"]))
        packet = json.loads(packet_path.read_text(encoding="utf-8"))
        original_digest = packet["packet_digest"]
        packet["slice"]["role_activation"] = "named_role_adapter_backed"
        packet["packet_digest"] = digest({key: value for key, value in packet.items() if key != "packet_digest"})
        packet_path.write_text(json.dumps(packet, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        self.assertNotEqual(original_digest, packet["packet_digest"])
        self.assertEqual(packet["packet_digest"], digest({key: value for key, value in packet.items()
                                                           if key != "packet_digest"}))
        return_path = Path(str(fixture["return_path"]))
        returned = json.loads(return_path.read_text(encoding="utf-8"))
        returned["activation"] = "adapter-backed"
        returned["admitted_packet_digest"] = packet["packet_digest"]
        return_path.write_text(json.dumps(returned, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        ledger_path = Path(str(fixture["ledger_path"]))
        ledger_before = ledger_path.read_bytes()
        failure = self.run_handoff(fixture, ok=False)
        self.assertRegex((failure.stderr + failure.stdout).lower(), r"dispatch|packet|digest")
        self.assertFalse(Path(str(fixture["handoff_path"])).exists())
        self.assertEqual(ledger_before, ledger_path.read_bytes())
        collection_path = Path(str(fixture["collection_path"]))
        self.assertFalse(collection_path.exists())

    def test_rejects_altered_live_dispatch_comment_body(self) -> None:
        fixture = self.make_fixture()
        self.write_live_issue(str(fixture["dispatch_body"]) + " ")
        with self.assertRaisesRegex(HANDOFF.ContractError, "dispatch|body|digest|readback"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_wrong_task_dispatch_comment_even_when_handoff_rehashed(self) -> None:
        fixture = self.make_fixture()
        payload = json.loads(canonical(fixture["dispatch_payload"]).decode("utf-8"))
        payload["task_uid"] = "task_" + "2" * 32
        wrong_task_body = self.dispatch_body(payload)
        self.write_live_issue(wrong_task_body)
        self.rebind_handoff_dispatch_body(fixture, wrong_task_body)
        with self.assertRaisesRegex(HANDOFF.ContractError, "dispatch|task|identity|body"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_dispatch_comment_without_live_admin_authority(self) -> None:
        fixture = self.make_fixture()
        self.write_live_issue(str(fixture["dispatch_body"]), permission="write")
        with self.assertRaisesRegex(HANDOFF.ContractError, "admin|permission|author|dispatch"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_duplicate_reciprocal_pr_issue_field_without_side_effects(self) -> None:
        fixture = self.make_fixture(create_handoff=False)
        data = json.loads(self.gh_data.read_text(encoding="utf-8"))
        data["issue"]["body"] += f"- pr_url: `https://github.com/{REPOSITORY}/pull/1`\n"
        self.gh_data.write_text(json.dumps(data, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        ledger_path = Path(str(fixture["ledger_path"]))
        ledger_before = ledger_path.read_bytes()
        handoff_path = Path(str(fixture["handoff_path"]))
        collection_path = Path(str(fixture["collection_path"]))

        failure = self.run_handoff(fixture, ok=False)
        self.assertRegex((failure.stderr + failure.stdout).lower(), r"pr binding|reciprocal|ambiguous")
        self.assertEqual(ledger_before, ledger_path.read_bytes())
        self.assertFalse(handoff_path.exists())
        self.assertFalse(collection_path.exists())

    def test_rejects_duplicate_task_uid_issue_field_without_side_effects(self) -> None:
        fixture = self.make_fixture(create_handoff=False)
        data = json.loads(self.gh_data.read_text(encoding="utf-8"))
        data["issue"]["body"] += f"task_uid: {TASK}\n"
        self.gh_data.write_text(json.dumps(data, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        ledger_path = Path(str(fixture["ledger_path"]))
        ledger_before = ledger_path.read_bytes()
        handoff_path = Path(str(fixture["handoff_path"]))
        collection_path = Path(str(fixture["collection_path"]))

        failure = self.run_handoff(fixture, ok=False)
        self.assertRegex((failure.stderr + failure.stdout).lower(), r"uid|identity|ambiguous")
        self.assertEqual(ledger_before, ledger_path.read_bytes())
        self.assertFalse(handoff_path.exists())
        self.assertFalse(collection_path.exists())

    def test_rejects_return_activation_conflicting_with_slice_packet(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_and_handoff(fixture, "activation", "named-role-adapter")
        with self.assertRaisesRegex(HANDOFF.ContractError, "activation|packet|metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_return_context_conflicting_with_slice_packet(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_and_handoff(fixture, "context_delivery", "full-history")
        with self.assertRaisesRegex(HANDOFF.ContractError, "context|packet|metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_return_runtime_conflicting_with_slice_packet(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_and_handoff(fixture, "actual_runtime", "gpt-6-astra: high")
        with self.assertRaisesRegex(HANDOFF.ContractError, "runtime|packet|metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_comparison_oid_different_from_source_review_scope(self) -> None:
        fixture = self.make_fixture()
        plan_path = Path(str(fixture["plan_path"]))
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
        plan["comparison_oid"] = "9" * 40
        plan_raw = json.dumps(plan, ensure_ascii=False, sort_keys=True).encode() + b"\n"
        plan_path.write_bytes(plan_raw)

        handoff_path = Path(str(fixture["handoff_path"]))
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        handoff["comparison_oid"] = plan["comparison_oid"]
        handoff["plan_sha256"] = hashlib.sha256(plan_raw).hexdigest()
        handoff["handoff_digest"] = digest({key: item for key, item in handoff.items() if key != "handoff_digest"})
        handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

        batch = json.loads(Path(str(fixture["batch_path"])).read_text(encoding="utf-8"))
        self.assertEqual(str(fixture["source_digest"]), batch["relevant_evidence_digest"])
        self.assertEqual(
            digest({key: batch[key] for key in ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}),
            batch["epoch"],
        )
        with self.assertRaisesRegex(HANDOFF.ContractError, "comparison|scope"):
            HANDOFF.validate_handoff(self.root, handoff_path)


if __name__ == "__main__":
    unittest.main()
