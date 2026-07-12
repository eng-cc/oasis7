#!/usr/bin/env python3
"""RED contracts for durable workflow writers and terminal recovery."""
from __future__ import annotations
import importlib.util, json, multiprocessing, os, re, subprocess, tempfile, unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]; PM=ROOT/"scripts/pm"
STORE=PM/"workflow-durable-store.py"; RECEIPTS=PM/"canonical-receipt-root.py"
FINALIZER=PM/"post-merge-finalize.py"; CLEANUP=PM/"post-merge-cleanup.sh"

def load(path: Path, name: str):
    spec=importlib.util.spec_from_file_location(name,path)
    if spec is None or spec.loader is None: raise AssertionError(f"cannot load {path}")
    mod=importlib.util.module_from_spec(spec); spec.loader.exec_module(mod); return mod

def merge_worker(mapping: str, uid: str) -> None:
    load(STORE,f"store_{os.getpid()}").merge_task_record(Path(mapping),uid,{"task_uid":uid,"writer":uid})

def production_writer_worker(module_path: str, mapping: str, uid: str, field: str) -> None:
    module=load(Path(module_path),f"writer_{os.getpid()}")
    module.persist_mapping(Path(mapping),{"version":1,"tasks":{uid:{"task_uid":uid,field:field}}})

def receipt_root_worker(repo: str, uid: str, queue: multiprocessing.Queue) -> None:
    result=subprocess.run(["python3",str(RECEIPTS),"--default-worktree",repo,
        "--task-uid",uid,"--create","--json"],text=True,capture_output=True)
    queue.put((result.returncode,result.stdout,result.stderr))

class MappingDurabilityContract(unittest.TestCase):
    def test_all_mapping_writers_use_one_path_scoped_atomic_store(self):
        self.assertTrue(STORE.is_file(),"missing shared durable mapping store")
        store=load(STORE,"workflow_durable_store")
        for api in ("mapping_lock_path","atomic_replace_json","merge_task_record"):
            self.assertTrue(callable(getattr(store,api,None)),f"missing {api}")
        for writer in (PM/"github-project-task.py",PM/"github-project-sync.py",PM/"github-project-workflow.py",FINALIZER):
            text=writer.read_text()
            self.assertIn("workflow-durable-store.py",text,f"{writer.name} bypasses shared store")
            self.assertNotRegex(text,r"(?m)^def save_mapping\(",f"{writer.name} retains private writer")

    def test_real_concurrent_merges_have_no_lost_update(self):
        self.assertTrue(STORE.is_file(),"missing shared durable mapping store")
        with tempfile.TemporaryDirectory() as td:
            mapping=Path(td)/"nested/tasks.json"; uids=[f"task_{i:032x}" for i in range(24)]
            ps=[multiprocessing.Process(target=merge_worker,args=(str(mapping),u)) for u in uids]
            for p in ps:p.start()
            for p in ps:p.join(10); self.assertEqual(0,p.exitcode)
            payload=json.loads(mapping.read_text())
            self.assertEqual(set(uids),set(payload["tasks"]))
            self.assertFalse(list(mapping.parent.glob("*.tmp*")))

    def test_sync_and_workflow_entrypoints_have_strict_no_lost_update(self):
        """Exercise production writer entrypoints, not only the shared helper."""
        with tempfile.TemporaryDirectory() as td:
            mapping=Path(td)/"tasks.json"; uid="task_"+"c"*32
            modules=[PM/"github-project-sync.py",PM/"github-project-workflow.py"]
            ps=[multiprocessing.Process(target=production_writer_worker,
                args=(str(modules[i%2]),str(mapping),uid,f"field_{i}")) for i in range(20)]
            for p in ps:p.start()
            for p in ps:p.join(15); self.assertEqual(0,p.exitcode)
            record=json.loads(mapping.read_text())["tasks"][uid]
            self.assertEqual({f"field_{i}" for i in range(20)},set(record)-{"task_uid"})

    def test_stale_same_task_snapshot_cannot_regress_or_delete_nested_authority(self):
        store=load(STORE,"monotonic_store")
        uid="task_"+"d"*32
        with tempfile.TemporaryDirectory() as td:
            mapping=Path(td)/"tasks.json"
            latest={"task_uid":uid,"repository":"org/repo","issue_number":7,
                "workflow_phase":"main_sync",
                "phase_receipts":{"main_sync":{"issuer":"sync","oid":"abc"}},
                "phase_receipt_sha256":{"main_sync":"digest"},
                "evidence":{"review":{"url":"https://example.invalid/1"}}}
            store.merge_task_record(mapping,uid,latest)
            stale={"task_uid":uid,"repository":"org/repo","issue_number":7,
                "workflow_phase":"pre_pr_ready","phase_receipts":{},
                "phase_receipt_sha256":{},"evidence":{}}
            store.merge_mapping_document(mapping,{"version":1,"tasks":{uid:stale}})
            record=json.loads(mapping.read_text())["tasks"][uid]
            self.assertEqual("main_sync",record["workflow_phase"])
            self.assertEqual(latest["phase_receipts"],record["phase_receipts"])
            self.assertEqual(latest["phase_receipt_sha256"],record["phase_receipt_sha256"])
            self.assertEqual(latest["evidence"],record["evidence"])

    def test_same_task_immutable_identity_conflict_fails_closed(self):
        store=load(STORE,"identity_store")
        uid="task_"+"e"*32
        with tempfile.TemporaryDirectory() as td:
            mapping=Path(td)/"tasks.json"
            store.merge_task_record(mapping,uid,{"task_uid":uid,"repository":"org/repo","issue_number":7})
            with self.assertRaises((ValueError,RuntimeError)):
                store.merge_task_record(mapping,uid,{"task_uid":uid,"repository":"evil/repo","issue_number":99})

    def test_production_persist_rejects_same_key_stale_regression_and_overwrite(self):
        """Production entrypoints may persist explicit patches/CAS, never stale incoming-wins snapshots."""
        uid="task_"+"9"*32
        with tempfile.TemporaryDirectory() as td:
            mapping=Path(td)/"tasks.json"
            latest={"version":1,"tasks":{uid:{"task_uid":uid,"repository":"org/repo",
                "issue_number":9,"pr_number":19,"status":"done","workflow_phase":"main_sync",
                "updated_at":"2026-07-12T10:00:00Z",
                "phase_receipts":{"main_sync":{"oid":"new"}},
                "phase_receipt_sha256":{"main_sync":"new-digest"}}}}
            stale={"version":1,"tasks":{uid:{"task_uid":uid,"repository":"org/repo",
                "issue_number":9,"pr_number":19,"status":"candidate","workflow_phase":"pre_pr_ready",
                "updated_at":"2026-07-11T10:00:00Z",
                "phase_receipts":{"main_sync":{"oid":"old"}},
                "phase_receipt_sha256":{"main_sync":"old-digest"}}}}
            sync=load(PM/"github-project-sync.py","sync_monotonic")
            workflow=load(PM/"github-project-workflow.py","workflow_monotonic")
            sync.persist_mapping(mapping,latest)
            workflow.persist_mapping(mapping,stale)
            record=json.loads(mapping.read_text())["tasks"][uid]
            self.assertEqual("done",record["status"])
            self.assertEqual("main_sync",record["workflow_phase"])
            self.assertEqual("new",record["phase_receipts"]["main_sync"]["oid"])
            self.assertEqual("new-digest",record["phase_receipt_sha256"]["main_sync"])
            self.assertEqual("2026-07-12T10:00:00Z",record["updated_at"])

class CleanupJournalContract(unittest.TestCase):
    def test_cleanup_uses_fsync_backed_atomic_journal(self):
        text=CLEANUP.read_text()
        self.assertIn("workflow-durable-store.py",text); self.assertIn("write-journal",text)
        self.assertNotRegex(text,r">\s*[\"']?\$?(?:INTENT|intent|journal)")

    def test_journal_recovers_from_each_commit_window(self):
        self.assertTrue(STORE.is_file(),"missing durable journal implementation")
        store=load(STORE,"journal_store")
        write=getattr(store,"atomic_journal_transition",None)
        recover=getattr(store,"recover_atomic_journal",None)
        self.assertTrue(callable(write),"missing atomic_journal_transition")
        self.assertTrue(callable(recover),"missing recover_atomic_journal")
        with tempfile.TemporaryDirectory() as td:
            path=Path(td)/"intent.json"; old={"state":"intent","revision":1}; new={"state":"action","revision":2}
            write(path,old)
            # Interrupted write/truncate windows may leave only an untrusted
            # sibling temp; recovery must keep the last committed authority.
            temp=path.with_name(path.name+".tmp.interrupted")
            temp.write_text('{"state":')
            self.assertEqual(old,recover(path))
            # A complete but uncommitted temp is not authoritative either.
            temp.write_text(json.dumps(new))
            self.assertEqual(old,recover(path))
            # After atomic replace (whether directory fsync completed or the
            # process died immediately after it), the authoritative file is
            # complete and recovery returns exactly the new revision.
            os.replace(temp,path)
            self.assertEqual(new,recover(path))

    def test_terminal_receipt_is_durable_before_journal_committed(self):
        text=CLEANUP.read_text()
        committed=text.index('terminal_receipt_committed\"]=True')
        prefix=text[:committed]
        self.assertRegex(prefix,r"(?s)(?:flush|fsync).*(?:replace|mv).*(?:fsync|sync-dir)",
            "terminal receipt needs file fsync, atomic replace, and parent directory fsync before committed")

class FinalizerLedgerContract(unittest.TestCase):
    def test_remote_effects_have_stable_durable_intent_action_readback_ledger(self):
        text=FINALIZER.read_text()
        for token in ("operation_id","intent","action","readback","committed","project_update","evidence_comment","issue_close"):
            self.assertIn(token,text)
        self.assertIn("workflow-durable-store.py",text)

    def test_finalizer_has_task_singleton_and_cas_bound_ledger_updates(self):
        text=FINALIZER.read_text()
        self.assertRegex(text,r"(?i)(task[_ -]?(?:scoped )?(?:lock|lease|singleton)|finalizer[_ -]lock)")
        self.assertRegex(text,r"(?i)(compare.and.swap|\bCAS\b|expected[_ -](?:revision|state)|revision[_ -]conflict)")
        self.assertNotRegex(text,r"old=durable_store\.recover_atomic_journal\(path\).*?atomic_journal_transition",
            "ledger read-modify-write must not occur outside one atomic transaction")

    def test_remote_readback_is_complete_and_issue_close_is_live_verified(self):
        text=FINALIZER.read_text()
        comment=text[text.index("def _reconcile_comment"):text.index("def _project_readback")]
        self.assertIn("--slurp",comment,"paginated gh api JSON must be slurped into one array")
        project=text[text.index("def _project_readback"):text.index("def fail")]
        self.assertRegex(project,r"(?i)(--paginate|hasNextPage|endCursor|pageInfo)",
            "Project readback must prove all pages were searched")
        self.assertNotIn('"readback",{"state":"CLOSED"}',text)
        close_pos=text.index('"issue","close"')
        self.assertIn('"issue","view"',text[close_pos:],"successful close must be followed by live issue readback")

    def test_created_comment_is_never_committed_without_paginated_live_body_reconcile(self):
        text=FINALIZER.read_text()
        create=text.index('"gh","issue","comment"')
        committed=text.index('"evidence_comment","committed"',create)
        window=text[create:committed]
        self.assertIn("_reconcile_comment",window,
            "a returned create URL is not readback; every create must be reconciled through paginated GET")
        self.assertRegex(window,r"(?i)(if not comment|fail\().*(?:reconcile|readback|matching)")
        reconcile=text[text.index("def _reconcile_comment"):text.index("def _project_readback")]
        for binding in ("Operation-ID", "issue", "body"):
            self.assertIn(binding,reconcile)
        self.assertIn("len(matches)!=1",reconcile)

class CanonicalReceiptRootContract(unittest.TestCase):
    def test_helper_derives_creates_and_rejects_cross_task_root(self):
        self.assertTrue(RECEIPTS.is_file(),"missing canonical receipt-root helper")
        uid="task_"+"a"*32; other="task_"+"b"*32
        with tempfile.TemporaryDirectory() as td:
            default=Path(td)/"repo"; default.mkdir(); subprocess.run(["git","init","-q",str(default)],check=True)
            got=subprocess.run(["python3",str(RECEIPTS),"--default-worktree",str(default),"--task-uid",uid,"--create","--json"],text=True,capture_output=True,check=True)
            root=Path(json.loads(got.stdout)["receipt_root"]); self.assertTrue(root.is_dir()); self.assertIn(uid,root.parts)
            bad=subprocess.run(["python3",str(RECEIPTS),"--default-worktree",str(default),"--task-uid",other,"--receipt-root",str(root),"--json"],text=True,capture_output=True)
            self.assertNotEqual(0,bad.returncode)

    def test_forty_concurrent_creators_all_succeed_with_one_atomic_identity(self):
        uid="task_"+"f"*32
        with tempfile.TemporaryDirectory() as td:
            default=Path(td)/"repo"; default.mkdir(); subprocess.run(["git","init","-q",str(default)],check=True)
            queue=multiprocessing.Queue()
            ps=[multiprocessing.Process(target=receipt_root_worker,args=(str(default),uid,queue)) for _ in range(40)]
            for p in ps:p.start()
            for p in ps:p.join(15); self.assertEqual(0,p.exitcode)
            results=[queue.get(timeout=2) for _ in ps]
            self.assertTrue(all(code==0 for code,_,_ in results),results)
            roots={json.loads(stdout)["receipt_root"] for _,stdout,_ in results}
            self.assertEqual(1,len(roots))
            root=Path(roots.pop()); identity=json.loads((root/"identity.json").read_text())
            self.assertEqual(uid,identity["task_uid"])
            self.assertFalse(list(root.glob("*.tmp*")))

    def test_identity_write_is_atomic_fsynced_and_repo_move_policy_is_explicit(self):
        text=RECEIPTS.read_text()
        self.assertRegex(text,r"(?i)(mkstemp|NamedTemporaryFile)")
        self.assertIn("fsync",text)
        self.assertRegex(text,r"os\.(?:replace|rename)")
        self.assertRegex(text,r"(?i)(git_common_dir.*(?:move|relocat|rotation|recover)|(?:move|relocat|rotation|recover).*git_common_dir)")

if __name__=="__main__":unittest.main(verbosity=2)
