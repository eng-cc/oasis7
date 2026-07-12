#!/usr/bin/env python3
"""Derive the task-bound durable receipt directory from Git common-dir."""
from __future__ import annotations
import argparse, fcntl, json, os, pathlib, re, subprocess, sys, tempfile

UID = re.compile(r"^task_[0-9a-f]{32}$")
ALLOWED={"merge-receipt.json","main-sync-receipt.json","terminal-cleanup-receipt.json","finalizer-ledger.json","cleanup-intent.json","patch-equivalence-receipt.json"}

def fail(message: str) -> None: raise SystemExit(f"canonical-receipt-root: {message}")

def main() -> int:
    p=argparse.ArgumentParser(); p.add_argument("--default-worktree",required=True); p.add_argument("--task-uid",required=True)
    p.add_argument("--receipt-root"); p.add_argument("--path"); p.add_argument("--name",choices=sorted(ALLOWED)); p.add_argument("--create",action="store_true"); p.add_argument("--json",action="store_true"); a=p.parse_args()
    if not UID.fullmatch(a.task_uid): fail("invalid task UID")
    wt=pathlib.Path(a.default_worktree).resolve()
    common=pathlib.Path(subprocess.check_output(["git","-C",str(wt),"rev-parse","--git-common-dir"],text=True).strip())
    if not common.is_absolute(): common=(wt/common).resolve()
    root=(common/"oasis7-workflow-receipts"/a.task_uid).resolve()
    if a.receipt_root and pathlib.Path(a.receipt_root).resolve()!=root: fail("receipt root is not canonical for task")
    metadata=root/"identity.json"
    # A git_common_dir move/relocation requires explicit trusted-mapping
    # rotation/rebind; ordinary creation must never silently mint new authority.
    expected={"schema":"oasis7_canonical_receipt_root_v1","task_uid":a.task_uid,"git_common_dir":str(common)}
    if a.create:
        root.mkdir(parents=True,exist_ok=True)
        lock_path=root/"identity.json.lock"
        with lock_path.open("a+b") as lock:
            fcntl.flock(lock.fileno(),fcntl.LOCK_EX)
            if metadata.exists() and json.loads(metadata.read_text())!=expected:
                fail("receipt root identity mismatch; repository moves require trusted mapping rotation/rebind")
            if not metadata.exists():
                fd,tmp_name=tempfile.mkstemp(prefix="identity.json.tmp.",dir=root)
                try:
                    with os.fdopen(fd,"w",encoding="utf-8") as out:
                        json.dump(expected,out,indent=2,sort_keys=True); out.write("\n")
                        out.flush(); os.fsync(out.fileno())
                    os.replace(tmp_name,metadata)
                    dir_fd=os.open(root,os.O_RDONLY|getattr(os,"O_DIRECTORY",0))
                    try: os.fsync(dir_fd)
                    finally: os.close(dir_fd)
                finally: pathlib.Path(tmp_name).unlink(missing_ok=True)
    elif root.exists() and metadata.exists() and json.loads(metadata.read_text())!=expected:
        fail("receipt root identity mismatch; repository moves require trusted mapping rotation/rebind")
    selected=root
    if a.path or a.name:
        requested=pathlib.Path(a.path).resolve() if a.path else root/a.name
        if requested.parent != root or requested.name not in ALLOWED or (a.name and requested.name != a.name): fail("path is outside canonical receipt layout")
        selected=requested
    result={"receipt_root":str(root),"path":str(selected),"task_uid":a.task_uid}
    print(json.dumps(result,sort_keys=True) if a.json else selected)
    return 0
if __name__=="__main__": raise SystemExit(main())
