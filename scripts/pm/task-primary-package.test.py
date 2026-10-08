#!/usr/bin/env python3
"""Real Git/Cargo facade completion with simulated external GitHub services."""
from __future__ import annotations
import argparse
import copy
import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import types
import sys
import unittest
from unittest.mock import patch

import task_primary_package as contract

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("primary_facade_test", Path(__file__).with_name("github-project-task.py"))
facade = importlib.util.module_from_spec(spec)
spec.loader.exec_module(facade)
UID = "task_" + "a" * 32
REPO = "eng-cc/oasis7"


class CompletionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        self.git("remote", "add", "origin", "git@github.com:eng-cc/oasis7.git")
        self.git("symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
        shutil.copytree(ROOT / "scripts", self.root / "scripts")
        self.write("doc/engineering/workflow/source-of-truth.md",
                   (ROOT / "doc/engineering/workflow/source-of-truth.md").read_text())
        (self.root / "scripts/product-doc-content-check.py").chmod(0o644)
        self.write("Cargo.toml", '[workspace]\nmembers=["crates/alpha"]\nresolver="2"\n')
        self.write("Cargo.lock", 'version = 3\n\n[[package]]\nname="alpha"\nversion="0.1.0"\n')
        self.write("crates/alpha/Cargo.toml", '[package]\nname="alpha"\nversion="0.1.0"\nedition="2021"\n')
        self.write("crates/alpha/src/lib.rs", 'pub fn alpha() {}\n')
        self.write(".pm/cargo-package-scope-policy.json", json.dumps({"schema": "oasis7-cargo-package-scope-policy/v1", "policy_version": 1, "protected_paths": [".pm/cargo-package-scope-policy.json"]}))
        self.git("add", "-A")
        self.git("commit", "-qm", "trusted fixture")
        self.base = self.git("rev-parse", "HEAD")
        self.git("switch", "-qc", "task/alpha")
        self.write("crates/alpha/src/lib.rs", 'pub fn alpha() { let _x = 1; }\n')
        self.git("add", "crates/alpha/src/lib.rs")
        self.git("commit", "-qm", "business")
        self.record = {"task_uid": UID, "repository": REPO, "issue_number": 12,
            "issue_url": f"https://github.com/{REPO}/issues/12", "project_item_id": "item",
            "owner_role": "runtime_engineer", "module": "engineering", "priority": "P2",
            "status": "committed", "workflow_phase": "execution", "canonical_worktree": str(self.root),
            "worktree_hint": str(self.root), "task_branch": "task/alpha", "default_branch": "main",
            "acceptance": ["Update alpha behavior within its existing package"], "bootstrap_epoch": 1}
        self.mapping = self.root / ".pm/github-project-sync/tasks.json"
        self.save()
        self.body = facade.issue_body(facade.task_from_record(UID, self.record))
        self.comments = []
        def evidence(comment_id, body):
            return {"id": comment_id, "html_url": self.record["issue_url"] + f"#issuecomment-{comment_id}",
                "issue_url": f"https://api.github.com/repos/{REPO}/issues/12", "body": body,
                "created_at": "2026-10-02T10:00:00Z", "updated_at": "2026-10-02T10:00:00Z", "user": {"login": "writer"}}
        scope = "\n".join(["## Plan-Gap Evidence", "step_id: alpha-change", "acceptance_refs: existing behavior",
            f"dependencies: {UID}", "verification_command: cargo test", "verification_evidence: RED",
            "write_scope: crates/alpha/src/lib.rs", "out_of_scope: all other paths", "required_role_slices: runtime_engineer"])
        freeze = "\n".join(["<!-- oasis7-pm-evidence -->", f"Task UID: {UID}",
            "Evidence Phase: draft_candidate_freeze", "Role: tpm", f"Source Worktree: {self.root}",
            "Source Branch: task/alpha", f"Source Head: {self.git('rev-parse', 'HEAD')}",
            "Comparison Ref: origin/main", f"Comparison OID: {self.base}"])
        self.scope_comments = {80: evidence(80, scope), 81: evidence(81, freeze)}
        selector = {"schema": contract.SELECTOR_SCHEMA, "task_uid": UID,
            "freeze": {"comment_id": 81, "body_sha256": "sha256:" + hashlib.sha256(freeze.encode()).hexdigest()},
            "authorizations": [{"comment_id": 80, "body_sha256": "sha256:" + hashlib.sha256(scope.encode()).hexdigest()}]}
        self.selector_path = self.root / ".pm/scope-selector.json"
        self.selector_path.write_text(json.dumps(selector))
        self.posts = 0
        self.edits = 0
        self.fail_edit = False
        self.uncertain_post = False
        self.permissions = True
        self.original_run = facade.run_text
        self.args = facade.build_parser().parse_args(["complete-primary-package", str(self.root),
            "--task-uid", UID, "--primary-package", "alpha", "--scope-base", self.base,
            "--scope-evidence-json", str(self.selector_path), "--json"])
        sync = facade.load_sync_module()
        sync.project_context = lambda *_: ("project", {})
        sync.recover_project_mapping_for_task_uids = self.project_readback
        for p in [patch.object(facade, "run_text", side_effect=self.external_command),
                  patch.object(facade, "load_sync_module", return_value=sync),
                  patch.object(facade, "project_refresh_graphql", side_effect=self.permission_readback),
                  patch.object(contract, "_github", side_effect=self.github)]:
            p.start()
            self.addCleanup(p.stop)

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True, stderr=subprocess.PIPE).strip()

    def write(self, path, text):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)

    def save(self):
        self.write(".pm/github-project-sync/tasks.json", json.dumps({"version": 1,
            "project": {"owner": "eng-cc", "number": 1, "id": "project"}, "tasks": {UID: self.record}}))

    def current(self):
        return json.loads(self.mapping.read_text())["tasks"][UID]

    def project_readback(self, *_):
        fields = facade.issue_task_fields(self.body)
        values = {"Task UID": UID, "Status": "In Progress", "PM Status": "committed",
            "Workflow Phase": "execution", "Owner Role": "runtime_engineer", "Module": "engineering",
            "Priority": "P2", "Canonical Worktree": str(self.root)}
        return {UID: {"project_item_id": "item", "project_field_values": values}}

    def permission_readback(self, *_args, **_kwargs):
        return {"data": {"repository": {"issue": {"number": 12, "url": self.record["issue_url"],
            "state": "OPEN", "viewerCanUpdate": self.permissions}},
            "node": {"project": {"viewerCanUpdate": self.permissions}}}}

    def github(self, repo, endpoint):
        self.assertEqual(REPO, repo)
        if "/collaborators/" in endpoint:
            return {"permission": "admin"}
        if "/issues/comments/" in endpoint:
            comment_id = int(endpoint.rsplit("/", 1)[1])
            return self.scope_comments[comment_id] if comment_id in self.scope_comments else next(c for c in self.comments if c["id"] == comment_id)
        return {"number": 12, "html_url": self.record["issue_url"], "body": self.body}

    def external_command(self, command):
        if command[0] != "gh":
            return self.original_run(command)
        if command[1:3] == ["issue", "list"]:
            return json.dumps([{"number": 12}])
        if command[1:3] == ["issue", "view"]:
            return json.dumps({"number": 12, "url": self.record["issue_url"], "body": self.body,
                               "title": "alpha task", "state": "OPEN", "updatedAt": "now"})
        if command[1:3] == ["issue", "comment"]:
            self.posts += 1
            body = Path(command[-1]).read_text()
            self.comments.append({"id": 99, "html_url": self.record["issue_url"] + "#issuecomment-99",
                "issue_url": f"https://api.github.com/repos/{REPO}/issues/12", "body": body,
                "created_at": "time", "updated_at": "time", "user": {"login": "writer"}})
            if self.uncertain_post:
                raise subprocess.TimeoutExpired(command, 180)
            return self.comments[-1]["html_url"]
        if command[1:3] == ["issue", "edit"]:
            self.edits += 1
            if self.fail_edit:
                raise subprocess.TimeoutExpired(command, 180)
            self.body = Path(command[-1]).read_text()
            return "edited"
        if command[1] == "api":
            endpoint = command[2]
            if endpoint.endswith("/comments"):
                return json.dumps([self.comments])
            return json.dumps(self.github(REPO, endpoint))
        raise AssertionError(command)

    def complete(self):
        return facade.command_complete_primary_package(self.args)

    def policy_authority_response(self, command, **kwargs):
        self.assertEqual(command[:2], ["gh", "api"])
        endpoint = command[2]
        if endpoint == f"repos/{REPO}":
            value = {"full_name": REPO, "default_branch": "main"}
        elif endpoint == f"repos/{REPO}/branches/main":
            value = {"name": "main", "protected": True, "commit": {"sha": self.base}}
        elif endpoint.startswith(f"repos/{REPO}/contents/"):
            relative, ref = endpoint.split("/contents/", 1)[1].split("?ref=", 1)
            self.assertEqual(ref, self.base)
            self.assertIn(relative, ("scripts/pm/loop-policy.v1.json", "doc/engineering/workflow/source-of-truth.md"))
            raw = subprocess.check_output(["git", "-C", str(self.root), "show", f"{ref}:{relative}"])
            value = {"encoding": "base64", "content": base64.b64encode(raw).decode()}
        else:
            raise AssertionError("unexpected policy authority request: " + repr(command))
        payload = json.dumps(value)
        return subprocess.CompletedProcess(command, 0,
            payload if kwargs.get("text") else payload.encode(),
            "" if kwargs.get("text") else b"")

    def test_real_scope_publication_idempotency_preserves_identity(self):
        before = copy.deepcopy(self.record)
        self.assertEqual(0, self.complete())
        current = self.current()
        self.assertEqual("alpha", contract.effective_primary_package(current))
        for key in before:
            self.assertEqual(before[key], current[key])
        self.assertEqual(0, self.complete())
        self.assertEqual((1, 1), (self.posts, self.edits))
        reference = contract.completion_reference(current)
        contract.validate_consumed_contracts([reference], current, root=self.root)
        for refs in ([], [reference, reference], [{**reference, "comment_id": 100}]):
            with self.assertRaises(ValueError):
                contract.validate_consumed_contracts(refs, current, root=self.root)

    def test_comment_only_partial_write_recovers_same_action_after_head_moves(self):
        self.fail_edit = True
        with self.assertRaises(subprocess.TimeoutExpired):
            self.complete()
        self.assertNotIn("primary_package", self.current())
        action = contract.parse_completion_body(self.comments[0]["body"])["action_id"]
        self.write("crates/alpha/src/lib.rs", "pub fn alpha() { let _x = 2; }\n")
        self.git("add", "crates/alpha/src/lib.rs")
        self.git("commit", "-qm", "later head")
        self.fail_edit = False
        self.complete()
        self.assertEqual(action, self.current()[contract.FIELD]["payload"]["action_id"])
        self.assertEqual(1, self.posts)

    def test_uncertain_post_reconciles_without_duplicate(self):
        self.uncertain_post = True
        with self.assertRaises(subprocess.TimeoutExpired):
            self.complete()
        self.uncertain_post = False
        self.complete()
        self.assertEqual(1, self.posts)

    def test_uncertain_missing_post_cannot_repost(self):
        self.uncertain_post = True
        with self.assertRaises(subprocess.TimeoutExpired):
            self.complete()
        self.comments.clear()
        self.uncertain_post = False
        with self.assertRaises(SystemExit):
            self.complete()
        self.assertEqual(1, self.posts)

    def test_null_supported_empty_and_existing_primary_rejected(self):
        self.record["primary_package"] = None
        self.save()
        self.complete()
        self.assertTrue(self.current()[contract.FIELD]["payload"]["before"]["primary_present"])
        self.args.primary_package = "beta"
        with self.assertRaises(SystemExit):
            self.complete()

    def test_acceptance_without_package_token_preserved(self):
        self.record["acceptance"] = ["Preserve the documented behavior and validate its trust boundary"]
        self.body = facade.issue_body(facade.task_from_record(UID, self.record))
        self.save()
        before = copy.deepcopy(self.record["acceptance"])
        self.assertEqual(0, self.complete())
        self.assertEqual(before, self.current()["acceptance"])

    def test_missing_scope_authority_does_not_publish(self):
        self.args.scope_evidence_json = None
        with self.assertRaises(SystemExit):
            self.complete()
        self.assertEqual((0, 0), (self.posts, self.edits))

    def test_scope_server_identity_digest_edit_and_date_fail_closed(self):
        selector = json.loads(self.selector_path.read_text())
        original = copy.deepcopy(self.scope_comments[80])
        for key, value in (("issue_url", "https://api.github.com/repos/eng-cc/oasis7/issues/13"),
                           ("updated_at", "2026-10-02T10:01:00Z"),
                           ("body", original["body"] + "\nsubstituted"),
                           ("created_at", "2026-10-03T10:00:00Z")):
            with self.subTest(key=key):
                self.scope_comments[80] = {**original, key: value}
                if key == "created_at":
                    self.scope_comments[80]["updated_at"] = value
                with self.assertRaises(ValueError):
                    contract.existing_scope_proof(self.root, self.record, selector, self.base, self.git("rev-parse", "HEAD"))
        self.assertEqual((0, 0), (self.posts, self.edits))

    def test_scope_author_must_have_current_admin_permission(self):
        selector = json.loads(self.selector_path.read_text())
        def denied(repo, endpoint):
            return {"permission": "write"} if "/collaborators/" in endpoint else self.github(repo, endpoint)
        with patch.object(contract, "_github", side_effect=denied), self.assertRaisesRegex(ValueError, "admin"):
            contract.existing_scope_proof(self.root, self.record, selector, self.base, "HEAD")

    def test_scope_selector_and_complete_clause_are_closed(self):
        selector = json.loads(self.selector_path.read_text())
        for bad in ({**selector, "extra": True}, {**selector, "task_uid": "task_" + "b" * 32},
                    {**selector, "authorizations": [selector["authorizations"][0]] * 2},
                    {**selector, "freeze": {**selector["freeze"], "comment_id": True}}):
            with self.subTest(selector=bad), self.assertRaises(ValueError):
                contract.validate_scope_selector(bad, UID)
        body = self.scope_comments[80]["body"]
        for suffix in (", ../outside.rs", ", crates/alpha/src/*.rs", " plus arbitrary paths"):
            changed = body.replace("write_scope: crates/alpha/src/lib.rs", "write_scope: crates/alpha/src/lib.rs" + suffix)
            heading, clause = contract._scope_clause(changed, UID)
            with self.assertRaises(ValueError):
                contract._scope_permissions(self.root, self.base, heading, clause, self.record, None)

    def test_new_endpoint_cannot_expand_original_frozen_scope(self):
        self.write("crates/alpha/src/new.rs", "pub fn new() {}\n")
        self.git("add", "crates/alpha/src/new.rs")
        self.git("commit", "-qm", "unapproved endpoint")
        with self.assertRaises(SystemExit):
            self.complete()
        self.assertEqual((0, 0), (self.posts, self.edits))

    def test_exclusions_subtract_exact_scope_and_unknown_prose_fails(self):
        selector = json.loads(self.selector_path.read_text())
        original = self.scope_comments[80]["body"]
        for exclusion in ("crates/alpha/src/lib.rs", "all sources unless operator approves"):
            body = original.replace("out_of_scope: all other paths", "out_of_scope: " + exclusion)
            self.scope_comments[80]["body"] = body
            selector["authorizations"][0]["body_sha256"] = "sha256:" + hashlib.sha256(body.encode()).hexdigest()
            with self.subTest(exclusion=exclusion), self.assertRaises(ValueError):
                contract.existing_scope_proof(self.root, self.record, selector, self.base, self.git("rev-parse", "HEAD"))

    def test_historical_v1_completion_remains_readable(self):
        self.complete()
        current = self.current()
        record = current[contract.FIELD]
        payload = record["payload"]
        del payload["scope_evidence"]
        payload["schema"] = contract.SCHEMA
        payload["action_id"] = contract.digest({key: value for key, value in payload.items() if key != "action_id"})
        body = contract.completion_body(payload)
        record["server"]["body_sha256"] = "sha256:" + hashlib.sha256(body.encode()).hexdigest()
        self.comments[0]["body"] = body
        self.body = facade.issue_body(facade.task_from_record(UID, current))
        contract.validate_current_completion(self.root, current)
        self.assertEqual(contract.SCHEMA, contract.completion_reference(current)["schema"])

    def test_real_canonical_loop_admission_and_nonrecursive_reader(self):
        tool = self.root / "effective-tools"
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        self.git("worktree", "add", "--detach", str(tool), self.base)
        policy_bytes = (tool / "scripts/pm/loop-policy.v1.json").read_bytes()
        binding = {"schema": "oasis7.loop-task/v1", "task_uid": UID, "owner_role": "runtime_engineer",
            "bootstrap_epoch": 1, "loop": "code", "change_id": "alpha", "manual_request_ref": "existing-request",
            "request_key": "alpha", "target_delivery": "alpha", "policy_commit": self.base,
            "policy_digest": "sha256:" + hashlib.sha256(policy_bytes).hexdigest(),
            "write_scope": ["crates/alpha/src/lib.rs"], "out_of_scope": [], "acceptance_refs": ["existing acceptance"],
            "dependencies": [], "input_contracts": []}
        self.record["loop_binding"] = binding
        self.body = facade.issue_body(facade.task_from_record(UID, self.record))
        self.save()
        self.args.scope_evidence_json = None
        original_output, original_run = subprocess.check_output, subprocess.run
        def output(command, **kwargs):
            if command[0] == "gh":
                if command[1:3] == ["api", f"repos/{REPO}/issues/12/comments?per_page=100"]:
                    self.assertEqual(command[3:], ["--paginate", "--slurp"])
                    # Complete live no-adoption history is a list of pages,
                    # including an empty terminal page before publication.
                    return json.dumps([self.comments])
                return json.dumps(self.github(REPO, command[2]))
            return original_output(command, **kwargs)
        def run(command, **kwargs):
            if command[0] == "gh":
                return self.policy_authority_response(command, **kwargs)
            if command[0] == "git" and "fetch" in command:
                return subprocess.CompletedProcess(command, 0, b"", b"")
            return original_run(command, **kwargs)
        with patch.dict("os.environ", {"OASIS7_LOOP_TOOL_ROOT": str(tool)}), patch.object(subprocess, "check_output", side_effect=output), patch.object(subprocess, "run", side_effect=run):
            binding["out_of_scope"] = ["crates/alpha/src/lib.rs"]
            self.body = facade.issue_body(facade.task_from_record(UID, self.record))
            self.save()
            with self.assertRaises(SystemExit):
                self.complete()
            self.assertEqual((0, 0), (self.posts, self.edits))
            binding["out_of_scope"] = []
            self.body = facade.issue_body(facade.task_from_record(UID, self.record))
            self.save()
            self.assertEqual(0, self.complete())
            current = self.current()
            self.assertEqual(contract.LOOP_PROOF_SCHEMA, current[contract.FIELD]["payload"]["scope_evidence"]["schema"])
            # Only external service adapters are simulated; policy, contracts, Git and Cargo run.
            with patch("loop_gate.admission", side_effect=AssertionError("reader entered full task admission")):
                contract.validate_current_completion(self.root, current)
            isolated = self.root / ".pm/isolated-services.json"
            authority = {}
            for endpoint in (f"repos/{REPO}", f"repos/{REPO}/branches/main",
                    f"repos/{REPO}/contents/scripts/pm/loop-policy.v1.json?ref={self.base}",
                    f"repos/{REPO}/contents/doc/engineering/workflow/source-of-truth.md?ref={self.base}"):
                authority[endpoint] = json.loads(self.policy_authority_response(["gh", "api", endpoint]).stdout)
            isolated.write_text(json.dumps({"task": current, "body": self.body, "comments": self.comments,
                                           "authority": authority}))
            program = '''
import base64, copy, importlib.util, json, os, re, subprocess, sys, types
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location("isolated_primary",sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
services=json.loads(Path(sys.argv[4]).read_text())
os.environ["OASIS7_LOOP_TOOL_ROOT"]=sys.argv[3]
poison=types.ModuleType("loop_gate")
poison.live_binding=lambda *_: (_ for _ in ()).throw(AssertionError("poisoned helper used"))
sys.modules["loop_gate"]=poison
def github(repo,endpoint):
    if "/collaborators/" in endpoint:return {"permission":"admin"}
    if "/issues/comments/" in endpoint:return next(c for c in services["comments"] if c["id"]==int(endpoint.rsplit("/",1)[1]))
    return {"number":services["task"]["issue_number"],"html_url":services["task"]["issue_url"],"body":services["body"]}
real_output,real_run=subprocess.check_output,subprocess.run
def output(command,**kwargs):
    if command[0]=="gh":
        if command[2].endswith("/comments?per_page=100"):
            assert command[3:]==["--paginate","--slurp"]
            return json.dumps([services["comments"]])
        return json.dumps(github("eng-cc/oasis7",command[2]))
    return real_output(command,**kwargs)
def run(command,**kwargs):
    if command[0]=="gh":
        assert command[:2]==["gh","api"] and command[2] in services["authority"],command
        payload=json.dumps(services["authority"][command[2]])
        return subprocess.CompletedProcess(command,0,payload if kwargs.get("text") else payload.encode(),"" if kwargs.get("text") else b"")
    if command[0]=="git" and "fetch" in command:return subprocess.CompletedProcess(command,0,b"",b"")
    return real_run(command,**kwargs)
with patch.object(module,"_github",side_effect=github),patch.object(subprocess,"check_output",side_effect=output),patch.object(subprocess,"run",side_effect=run):
    def forbid_full_admission(frame,event,arg):
        if event=="call" and frame.f_code.co_name in {"admission","validate_task"}:
            raise AssertionError("current reader entered Task admission graph")
    sys.setprofile(forbid_full_admission)
    module.validate_current_completion(Path(sys.argv[2]),services["task"])
    original_body=services["body"]
    for field in ("task_uid","owner_role","bootstrap_epoch"):
        for missing in (False,True):
            changed=copy.deepcopy(services["task"])
            if missing:
                changed["loop_binding"].pop(field)
                changed.pop(field)
            else:
                changed["loop_binding"][field] = (2 if field=="bootstrap_epoch" else
                    "task_"+"f"*32 if field=="task_uid" else "blockchain_ops_engineer")
            encoded=base64.b64encode(json.dumps(changed["loop_binding"]).encode()).decode()
            services["body"]=re.sub(r"(- loop_binding_b64: `)[^`]+(`)",lambda m:m[1]+encoded+m[2],original_body)
            try:
                module.loop_scope_proof(Path(sys.argv[2]),changed,package="alpha",current_authority=True)
            except ValueError as error:
                assert "identity mismatch: "+field in str(error),str(error)
            else:
                raise AssertionError("isolated reader accepted identity substitution: "+field)
    services["body"]=original_body
    sys.setprofile(None)
assert sys.modules["loop_gate"] is poison
'''
            result = original_run([sys.executable, "-I", "-c", program, str(Path(contract.__file__).resolve()),
                                   str(self.root), str(tool), str(isolated)], capture_output=True, text=True)
            self.assertEqual(0, result.returncode, result.stderr)
            self.body = self.body.replace("- loop_binding_b64:", "- deleted_loop_binding_b64:")
            with self.assertRaises(ValueError):
                contract.validate_current_completion(self.root, current)

    def test_isolated_helper_closure_rejects_missing_and_foreign_siblings(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            module_path = root / "task_primary_package.py"
            shutil.copyfile(contract.__file__, module_path)
            spec = importlib.util.spec_from_file_location("isolated_missing_primary", module_path)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            for foreign in (False, True):
                if foreign:
                    (root / "loop_recovery.py").symlink_to(Path(contract.__file__).with_name("loop_recovery.py"))
                with self.subTest(foreign=foreign), self.assertRaisesRegex(ValueError, "missing or unsafe"):
                    module.loop_scope_proof(self.root, self.record, package="alpha", current_authority=True)

    def test_legacy_write_scope_family_cannot_cross_heading(self):
        clause = "Ops ONLY scripts/local-signer/tests/test_runtime_gate_contract.py plus unique UID scratch RED report/log. Runtime architecture/contract report remains scratch-only for now."
        for heading in ("## Plan-Gap Evidence — PHASE 2 GREEN implementation release",
                        "## Plan-Gap Evidence — required signer documentation object metadata sync"):
            with self.subTest(heading=heading), self.assertRaises(ValueError):
                contract._scope_permissions(self.root, self.base, heading, clause, self.record, None)

    def test_missing_issue_project_permission_cannot_publish(self):
        self.permissions = False
        with self.assertRaises(SystemExit):
            self.complete()
        self.assertEqual((0, 0), (self.posts, self.edits))

    def test_actual_shared_snapshot_validator_preserves_saved_bytes(self):
        spec = importlib.util.spec_from_file_location("primary_snapshot_test", ROOT / "scripts/pm/bootstrap-task-snapshot.py")
        snapshot = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(snapshot)
        saved_path = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        args = argparse.Namespace(repo_root=str(self.root), tasks_json=str(self.mapping), snapshot=str(saved_path),
            task_uid=UID, request_identity="fixed-request", producer="test")
        snapshot.create(args)
        saved = saved_path.read_bytes()
        self.complete()
        with patch.object(snapshot.primary_contract, "_github", side_effect=self.github):
            self.assertEqual(saved_path, snapshot.validate(args))
            self.assertEqual(saved_path, snapshot.validate_epoch_identity(args))
            self.assertEqual(saved, saved_path.read_bytes())
            altered = self.current()
            altered["acceptance"] = ["Different alpha task"]
            self.write(".pm/github-project-sync/tasks.json", json.dumps({"version": 1,
                "project": {"owner": "eng-cc", "number": 1, "id": "project"}, "tasks": {UID: altered}}))
            with self.assertRaises(snapshot.SnapshotError):
                snapshot.validate_epoch_identity(args)

    def test_projection_loads_complete_target_authority_when_target_differs_from_merge_base(self):
        spec = importlib.util.spec_from_file_location("primary_projection_target_test", ROOT / "scripts/pm/workflow-impact-projection.py")
        projection = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(projection)
        head = self.git("rev-parse", "HEAD")
        self.git("switch", "-q", "main")
        self.write("README.md", "unrelated target advancement\n")
        self.git("add", "README.md")
        self.git("commit", "-qm", "target B")
        target = self.git("rev-parse", "HEAD")
        self.git("switch", "-q", "task/alpha")
        value = {"task_uid": UID, "source_head_oid": head, "scope_base_oid": self.base,
            "changed_paths": ["crates/alpha/src/lib.rs"], "change_class": "mixed",
            "manual_roles": ["repository_health_engineer"], "domain_role": None, "test_profile": "required",
            "declared_tests": ["required_gate_baseline"], "consumed_contracts": [],
            "public_semantics": [], "affected_consumers": [],
            "closure_status": {"status": "incomplete", "reason": "scope fixture", "evidence": []}}
        actual = projection.build_projection(self.root, value, planner_authority_oid=target)
        self.assertEqual(self.base, actual["scope_base_oid"])
        self.assertNotEqual(self.base, target)
        self.assertTrue(actual["planner_config_sha256"])

    def test_strict_primary_and_issue_json_parser(self):
        for value in ("", " alpha", "alpha ", 1, False, [], {}):
            with self.assertRaises(ValueError):
                contract.strict_primary_package(value)
        for body in ("- primary_package: `alpha`\n- primary_package: `alpha`\n",
                     "- primary_package: ``\n", "primary_package: null\n"):
            with self.assertRaises(ValueError):
                contract.issue_primary_fields(body)
        with self.assertRaises(ValueError):
            contract.strict_json('{"a":null,"a":1}')

    def test_history_snapshot_only_primary_exception_and_server_authority(self):
        self.complete()
        current = self.current()
        saved = {"primary_package": None, "owner_role": "runtime_engineer"}
        new = {**saved, "primary_package": "alpha"}
        self.assertTrue(contract.snapshot_primary_compatible(saved, new, current))
        altered = copy.deepcopy(current)
        altered["owner_role"] = "viewer_engineer"
        with self.assertRaises(ValueError):
            contract.snapshot_primary_compatible(saved, new, altered)
        self.comments[0]["updated_at"] = "edited"
        with self.assertRaises(ValueError):
            contract.validate_current_completion(self.root, current)


ORIGINAL_SCOPE_FIXTURE = json.loads("{\"task\":{\"task_uid\":\"task_7f6e4e83a3c44cc3858e92567f925e4c\",\"repository\":\"eng-cc/oasis7\",\"issue_number\":4245,\"issue_url\":\"https://github.com/eng-cc/oasis7/issues/4245\",\"project_item_id\":\"PVTI_lAHOALIiks4Bb-Wtzg-GS_g\",\"owner_role\":\"blockchain_ops_engineer\",\"canonical_worktree\":\"/Users/scc/ccwork/oasis7/.worktrees/oasis7-engineering-local-signer-fixed-clt-runtime\",\"task_branch\":\"task/engineering-local-signer-fixed-clt-runtime\",\"default_branch\":\"main\",\"source_refs\":[\"doc/p2p/blockchain/local-file-signing-backend.design.md\"],\"acceptance\":[\"Define and implement an externally approved prelaunch trust gate for the fixed direct CLT Python executable and its framework resources before any root Python code runs\",\"Launch the approved runtime through a cleared environment with -I -S and bind its exact identity consistently through bootstrap, host plan, apply and unchanged verification\",\"Preserve fail-closed behavior for wrong signatures, mutable or ACL-bearing ancestry, symlink or identity substitution, unexpected imports and runtime drift; prove deterministic RED and independent GREEN\",\"Update package provenance, design and runbook coherently and obtain involved-role review without host installation, key creation, signing, mount-policy or jobs-layout changes\"]},\"comments\":[{\"id\":5951822314,\"html_url\":\"https://github.com/eng-cc/oasis7/issues/4245#issuecomment-5951822314\",\"issue_url\":\"https://api.github.com/repos/eng-cc/oasis7/issues/4245\",\"body\":\"<!-- oasis7-pm-evidence -->\\nTask UID: task_7f6e4e83a3c44cc3858e92567f925e4c\\nEvidence Phase: draft_candidate_freeze\\nRole: tpm\\nRecorded At: 2026-10-02T11:55:34.640795+00:00\\n\\nSource Worktree: /Users/scc/ccwork/oasis7/.worktrees/oasis7-engineering-local-signer-fixed-clt-runtime\\nSource Branch: task/engineering-local-signer-fixed-clt-runtime\\nSource Head: 41c4a4d35afcbc5ae94bef035941c3edfa9ca146\\nComparison Ref: origin/main\\nComparison OID: 735fae2c5d9042657ff353eb92df5a8fa2d5afd4\\n\",\"created_at\":\"2026-10-02T11:55:36Z\",\"updated_at\":\"2026-10-02T11:55:36Z\",\"user\":{\"login\":\"eng-cc\"}},{\"id\":5950468540,\"html_url\":\"https://github.com/eng-cc/oasis7/issues/4245#issuecomment-5950468540\",\"issue_url\":\"https://api.github.com/repos/eng-cc/oasis7/issues/4245\",\"body\":\"## Plan-Gap Evidence — PHASE 2 GREEN implementation release\\n\\nstep_id: fixed-clt-native-and-bootstrap-green\\nacceptance_refs: Issue4245 source repair, accepted RED5950206900, architecture5949636273, QA5950101009. Canonical design author has updated runtime gate authority first; joint Ops/Runtime agreement received for fixed CLI and FD3 schema. Immutable RED c5b38ce27cb00b502e51ffdae12ee92f86c9ba75fb5c90b4d90a8511e79e1a8f remains unchanged.\\ndependencies: exact UID task_7f6e4e83a3c44cc3858e92567f925e4c/canonicalworktree/HEAD37e30b9a221e6faed3ab5536364bb5074141d170; bound audit ok. Ops finishes minor schema/source-wording corrections before Python edits, Runtime implements agreed existing authority. This is source implementation only.\\nverification_command: Python unittest discover scripts/local-signer/tests; env -u RUSTC_WRAPPER cargo test -p oasis7_local_signer including runtime gate target; cargo clippy -p oasis7_local_signer --all-targets -- -D warnings; cargo fmt --check; actual unprivileged production-verifier Darwin probe. Commands via rtk proxy, logs bounded in UID scratch; shared Cargo target retained.\\nverification_evidence: each author reports changedfiles/testoutput/artifactdigest and immutableREDhash; QA independently reruns native ABI negatives/sequencing and Python pre-capture/repeat identity cases, actual Darwin verifier read-only evidence before GREEN acceptance. No fake host result establishes deployment acceptance.\\nwrite_scope: Runtime owns crates/oasis7_local_signer/src/runtime_gate* and src/bin/oasis7_local_signer_runtime_gate.rs plus required Cargo.toml/lib.rs hooks/native cfg(test) tests. Ops owns scripts/local-signer/{install-release.py,installer.py,macos_host.py,package-release.py}, existing test_release_installer.py fixture updates/new focusedtests (not immutableRED), two local-file-signing-backend design/runbook docs. Agree separate gate packaging/build output with closed candidate manifest preserved unless explicitly documented provenance extension. No overlap/revert others.\\nout_of_scope: host install/accounts/jobs/mounts/keys/signing, workflow/helper/CI changes, credentials, third_party edits, unrelated runtime.\\nrequired_role_slices: Ops implementation; Runtime native implementation; independent QA after author returns, fresh nonauthor Ops/Runtime/QA plus involved review before PR. Integration order: design first already authored; joint protocol below; disjoint code; independent verification; freeze/review/lifecycle.\\nmandatory_context_checklist: user code-change authorization; exact task identity/authority and source HEAD; AGENTS source-of-truth/pinned Rust guidance; taskIssue4245; signer repo scope; collaboration disjoint authors/test immutability and adapter inactive/inherit parent configuration.\\n\\n### Frozen shared interface\\nLaunch env-i PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C LC_ALL=C approved gate --bootstrap-source protectedfile --expected-bootstrap-sha256 independenthex64 -- plan|apply args. Fixed direct CLTPython3.9 -I -S -B -c capturedbootstrap. FD3 readonly FIFO euid-owned, producer clearCLOEXEC/writeclose; bounded4096bytes UTF8ASCII compact sorted-keyJSON plus exactly1LF; consumer cap4097/readEOF/close/reject duplicates/extras/types/canonical deviations before stagecapture. All24fields strings: apple_anchor,bootstrap_sha256,dependency_policy_id,framework_cdhash,framework_dev,framework_ino,framework_mode,framework_path,framework_resource_seal,framework_uid,os_build,policy_id,requirement_id,runtime_arch,runtime_cdhash,runtime_dev,runtime_ino,runtime_mode,runtime_path,runtime_uid,runtime_version,schema_version,system_volume_trust. Constants apple/clt-python39-apple-dyld-v1/clt-python3.9-v1/com.apple.python3/valid/3.9/oasis7.local-signer.runtime-attestation.v1/current-booted-apple-os; fixedCLT paths; exact runtimeCDHash77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf frameworkCDHasha43551195b8d2eefd9356d81c1098ffc9e0a8b47. Dev/inopositive canonicaldecimal, UID0, protectedmode, archarm64|x86_64, OSbuildboundedASCII, bootstrapdigesthex64. Runtime verifies actual Apple/resources/dependency exactallowlist + native heldFD/name/ACL/fstatfs prepost then directexec; bootstrap validates runningfixedsysidentity/isolation/no_site/dontwritebytecode/importroots. Pipe record is transport not cryptographic attestation: trusted approved gate/bootstrap invocation and ApplebootedOS/dyld premise, no rootconcurrency atomicity claim. Plan/apply/complete-repeat compare exactruntimeidentity beforeeffects/readback; candidate manifest neverauthorizes gate.\\n\",\"created_at\":\"2026-10-02T10:33:02Z\",\"updated_at\":\"2026-10-02T10:33:02Z\",\"user\":{\"login\":\"eng-cc\"}},{\"id\":5949822388,\"html_url\":\"https://github.com/eng-cc/oasis7/issues/4245#issuecomment-5949822388\",\"issue_url\":\"https://api.github.com/repos/eng-cc/oasis7/issues/4245\",\"body\":\"## Plan-Gap Evidence: PHASE 1 RED authorization\\nArchitecture agreement consumes 5949636273 and Runtime report runtime-prelaunch-design-review.md. Source identity is current 37e30b9a221e6faed3ab5536364bb5074141d170 / UID task_7f6e4e83a3c44cc3858e92567f925e4c / canonical fixed-clt-runtime worktree.\\nstep_id: red-2\\nacceptance_refs: #4245 pre-Python gate and failclosed tests; independent bootstrap/gate provenance and runtime plan/apply/repeat binding.\\ndependencies: Ops/Runtime native-gate consensus, source-contract-1 evidence, pinned Rust guidance read, no production edits.\\nverification_command: rtk proxy env TMPDIR=/private/tmp python3 -m unittest discover -s scripts/local-signer/tests -p test_runtime_gate_contract.py -v; author may additionally select dedicated new installer runtime-admission tests in that same file.\\nverification_evidence: immutable test SHA256 and full RED log in unique UID scratch; failures must identify actual direct-Python/operator or missing runtime-proof admission behavior.\\nwrite_scope: Ops ONLY scripts/local-signer/tests/test_runtime_gate_contract.py plus unique UID scratch RED report/log. Runtime architecture/contract report remains scratch-only for now.\\nout_of_scope: all production/docs/Cargo changes during RED, all host/root/credential/key/signing changes, mount/jobs/workflow changes, old test weakening.\\nrequired_role_slices: Ops implementation owner authors RED; Runtime independently reviews native architecture and records whether native entrypoint has a pre-existing stable RED harness; independent QA GREEN and frozen nonauthor reviews follow.\\nNative harness boundary: newly introduced gate has no existing executable/API to exercise. A compile-only missing-symbol stub or placeholder/source-text mirror is not accepted as native behavior proof. If Runtime confirms no stable native RED surface, record that scoped skip reason; native cfg(test) backend negative matrix must still be implemented and independently verified during GREEN. Existing Python harness and real operator invocation block supply actual RED for this behavior repair.\\nExecutable contract for tests: the operator invokes only a separately approved/staged native gate, through fixed /usr/bin/env -i BEFORE native code loads. Native gate approval digest and bootstrap expected digest are external inputs, not candidate manifest-selected identity or self-approval. Native gate validates fixed CLT signed/resource identity and protected descriptor chain before launching exact direct runtime with -I -S -c; non-env inherited descriptor proof is mandatory. Apply and complete repeat compare runtime identity before any effects/early return. Immutable tests may assert this operator block plus fixture-only admission behaviors; no root/subprocess production seam in tests.\\nSame mandatory context checklist and role runtime convention as 5949397911, corrected by 5949453970: read current AGENTS/RTK/Ops or Runtime role/workflow/Issue/bootstrap/two signer docs/current source+test harness and #4244 proof. Not alone/no reverting, no main/third_party edits, shared Cargo cache unchanged. Return RED only and stop; TPM must record acceptance and explicit PHASE2 release before production. File/test contracts and skip matrix are immutable until a newly evidenced requirement correction; no silent weakening.\\n\",\"created_at\":\"2026-10-02T10:00:42Z\",\"updated_at\":\"2026-10-02T10:00:42Z\",\"user\":{\"login\":\"eng-cc\"}},{\"id\":5951725995,\"html_url\":\"https://github.com/eng-cc/oasis7/issues/4245#issuecomment-5951725995\",\"issue_url\":\"https://api.github.com/repos/eng-cc/oasis7/issues/4245\",\"body\":\"## Plan-Gap Evidence — required signer documentation object metadata sync\\nstep_id: signer-doc-corpus-object-sync\\nacceptance_refs: Issue4245 authorized signer design/runbook source repair; newly integrated main735fae2c introduced mandatory document-corpus v3 object tracking. RH readonly current locate found both signer docs indexed as objects only/no semantic records and stale content hashes after authorized edits.\\ndependencies: clean currentHEADc01ccc822639afc0c24861fdaae8b19dc3cd155a; original13sourcehashes unchanged by targetmerge; currentDCIv3 design§5.1/CLI§6.6 and helperhelp mustconfirm before action. Existingc01Freeze intent superseded ifderivedobjectwritechangeshead; neveroverwriteimmutablefreeze/evidence.\\nverification_command: current scripts/document-corpus-inventory.py sync --path exactsignerdesign --path exactsignerrunbook --apply (confirmhelpfirst); inspectgitdiffonlycorrespondingobjectshards; rtk proxy ./scripts/doc-governance-check.sh --full-corpus; changedprofessional-system-designpreflight withsupportedinputs; diffcheck. No syncsemantic/reviewpolicy modifications.\\nverification_evidence: uniqueUIDscratch syncoutput/log/digest and exacttwoobjectshard paths/currentcontenthashes; freshfullcorpus result; RH reviewmetadataonlydelta; TPM separatecommit/newfreeze/projectioncurrenthead afterrequiredrefresh.\\nwrite_scope: blockchain_ops_engineer may run canonical inventory sync ONLY for doc/p2p/blockchain/local-file-signing-backend.design.md and .runbook.md; resultingtwo objectrecord shards in doc/.governance/document-corpus/objects/ only plusUIDscratch. No signer source/docs edits in this correction; no metadataoutsideexacttwoobjects. Task source scope adds these necessaryderivedobjectrecords, not newpolicy/source-of-truth changes.\\nout_of_scope: semanticentrycreation/overrides/reviewexpiry/helper/policy/CI changes; unrelatedcorpusobjects; inventorybulkregeneration; cache/journal/credentials/hosteffects/rootinstall/keys/signing.\\nrequired_role_slices: RH diagnosed/qualifies exacthelperread-only; Ops boundedcanonicalmetadata syncauthor; RH validatesexactobjectdelta/corpuspreflight. QA sourcechecks continueunchanged13blobs; anysourcechangedforcesfreshreview/verification.\\nintegration_order: helperhelpqualification, exacttwoobjectsync, metadatareview/currentdocchecks, TPMcommitnewHEAD/newFreeze; conservativeprojectionfull/manual5roles/formalreview and exacttargetCI later.\\nmandatory_context_checklist: UIDtask_7f6e4e83a3c44cc3858e92567f925e4c/canonicalworktree/currentHEAD; userinterpreterrepairintent/AGENTS/currentDCIauthority; Issue4245singletruth/host4244pending; deriveddocrecordsnotworkflowpolicy; disjointsourceownership/noattestationorreadinessfabrication.\\n\",\"created_at\":\"2026-10-02T11:48:43Z\",\"updated_at\":\"2026-10-02T11:48:43Z\",\"user\":{\"login\":\"eng-cc\"}}]}")


class OriginalAuthorizationTests(unittest.TestCase):
    def test_original_fifteen_authorized_but_family_expansion_and_source_substitution_rejected(self):
        base = "735fae2c5d9042657ff353eb92df5a8fa2d5afd4"
        head = "41c4a4d35afcbc5ae94bef035941c3edfa9ca146"
        if subprocess.run(["git", "-C", str(ROOT), "cat-file", "-e", head], capture_output=True).returncode:
            self.skipTest("historical original Task Git objects unavailable")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            subprocess.run(["git", "clone", "--shared", "--no-checkout", str(ROOT), str(root)], check=True, capture_output=True)
            subprocess.run(["git", "-C", str(root), "checkout", "-qb", "task/engineering-local-signer-fixed-clt-runtime", head], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "Test"], check=True)
            fixture = copy.deepcopy(ORIGINAL_SCOPE_FIXTURE)
            task = fixture["task"]
            old_root = task["canonical_worktree"]
            task["canonical_worktree"] = str(root)
            task["worktree_hint"] = str(root)
            comments = {c["id"]: c for c in fixture["comments"]}
            comments[5951822314]["body"] = comments[5951822314]["body"].replace(old_root, str(root))
            locators = [{"comment_id": c["id"], "body_sha256": "sha256:" + hashlib.sha256(c["body"].encode()).hexdigest()} for c in fixture["comments"]]
            selector = {"schema": contract.SELECTOR_SCHEMA, "task_uid": task["task_uid"], "freeze": locators[0], "authorizations": locators[1:]}
            def service(repo, endpoint):
                if "/collaborators/" in endpoint:
                    return {"permission": "admin"}
                if "/issues/comments/" in endpoint:
                    return comments[int(endpoint.rsplit("/", 1)[1])]
                return {"number": task["issue_number"], "html_url": task["issue_url"], "body": facade.issue_body(facade.task_from_record(task["task_uid"], task))}
            acceptance = copy.deepcopy(task["acceptance"])
            with patch.object(contract, "_github", side_effect=service):
                proof = contract.existing_scope_proof(root, task, selector, base, head)
                self.assertEqual(15, len(proof["endpoints"]))
                self.assertEqual(acceptance, task["acceptance"])
                path = root / "crates/oasis7_local_signer/src/runtime_gate/new_unapproved.rs"
                path.write_text("pub fn unapproved() {}\\n")
                subprocess.run(["git", "-C", str(root), "add", str(path)], check=True)
                subprocess.run(["git", "-C", str(root), "commit", "-qm", "same-family expansion"], check=True)
                with self.assertRaisesRegex(ValueError, "expands"):
                    contract.existing_scope_proof(root, task, selector, base, "HEAD")
                task["source_refs"] = ["doc/engineering/workflow/source-of-truth.md"]
                with self.assertRaisesRegex(ValueError, "source references"):
                    contract.existing_scope_proof(root, task, selector, base, head)
                task["source_refs"] = ["doc/p2p/blockchain/local-file-signing-backend.design.md"]
                subprocess.run(["git", "-C", str(root), "checkout", "--detach", base], check=True, capture_output=True)
                duplicate = root / "doc/another/local-file-signing-backend.design.md"
                duplicate.parent.mkdir(parents=True)
                duplicate.write_text((root / task["source_refs"][0]).read_text())
                subprocess.run(["git", "-C", str(root), "add", str(duplicate)], check=True)
                subprocess.run(["git", "-C", str(root), "commit", "-qm", "ambiguous document fixture"], check=True)
                duplicate_base = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
                heading, clause = contract._scope_clause(comments[5950468540]["body"], task["task_uid"])
                with self.assertRaisesRegex(ValueError, "ambiguous"):
                    contract._scope_permissions(root, duplicate_base, heading, clause, task, contract._trusted_corpus(root, duplicate_base))

if __name__ == "__main__":
    unittest.main()
