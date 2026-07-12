#!/usr/bin/env python3
"""RED contract: every terminal helper accepts only canonical task receipts."""
import json, os, subprocess, tempfile, unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]; PM=ROOT/"scripts/pm"

class CanonicalTerminalPaths(unittest.TestCase):
    def test_all_helpers_validate_canonical_root_before_effects(self):
        expectations={
            "post-merge-main-sync.sh":("merge-receipt.json","main-sync-receipt.json","git -C \"$REPO_ROOT\" fetch"),
            "post-merge-cleanup.sh":("merge-receipt.json","main-sync-receipt.json","worktree remove"),
            "post-merge-finalize.py":("terminal-cleanup-receipt.json","finalizer-ledger.json","subprocess.check_output"),
        }
        for name,(input_name,product_name,effect) in expectations.items():
            text=(PM/name).read_text()
            with self.subTest(helper=name):
                self.assertIn("canonical-receipt-root.py",text)
                self.assertIn(input_name,text); self.assertIn(product_name,text)
                self.assertLess(text.index("canonical-receipt-root.py"),text.index(effect))

    def test_finalizer_rejects_arbitrary_external_absolute_receipt_before_gh(self):
        uid="task_"+"d"*32
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); repo=root/"repo"; task=root/"task"; external=root/"arbitrary"; log=root/"gh.log"
            repo.mkdir(); task.mkdir(); external.mkdir(); subprocess.run(["git","init","-q","-b","main",str(repo)],check=True)
            mapping=repo/".pm/github-project-sync/tasks.json"; mapping.parent.mkdir(parents=True)
            mapping.write_text(json.dumps({"tasks":{uid:{"task_uid":uid,"repository":"fixture/repo","canonical_worktree":str(task),"issue_number":1,"pr_number":2,"workflow_phase":"main_sync","merge_receipt":{"state":"MERGED"},"phase_receipts":{"main_sync":{"receipt_type":"oasis7_main_sync"}}}}}))
            receipt=external/"terminal-cleanup-receipt.json"
            receipt.write_text(json.dumps({"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup","task_uid":uid,"repository":"fixture/repo","issue_number":1,"pr_number":2}))
            bindir=root/"bin"; bindir.mkdir(); gh=bindir/"gh"
            gh.write_text(f"#!/bin/sh\necho \"$*\" >>{log}\nexit 99\n"); gh.chmod(0o755)
            result=subprocess.run(["python3",str(PM/"post-merge-finalize.py"),"--repo-root",str(repo),"--task-uid",uid,"--terminal-receipt",str(receipt)],env={**os.environ,"PATH":f"{bindir}:{os.environ['PATH']}"},capture_output=True,text=True)
            self.assertNotEqual(0,result.returncode)
            self.assertFalse(log.exists(),"finalizer reached GitHub before rejecting noncanonical receipt root")
            self.assertIn("canonical",result.stderr.lower())

if __name__=="__main__":unittest.main(verbosity=2)
