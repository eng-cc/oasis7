#!/usr/bin/env python3
"""Finalize one receipt-proven terminal workflow transition idempotently."""
from __future__ import annotations
import argparse, fcntl, hashlib, importlib.util, json, os, pathlib, re, subprocess, sys, tempfile, urllib.parse

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
CANONICAL_ROOT_HELPER = SCRIPT_DIR/"canonical-receipt-root.py"
_store_spec=importlib.util.spec_from_file_location("workflow_durable_store",SCRIPT_DIR/"workflow-durable-store.py")
assert _store_spec and _store_spec.loader
durable_store=importlib.util.module_from_spec(_store_spec); _store_spec.loader.exec_module(durable_store)
_workflow_spec=importlib.util.spec_from_file_location("github_project_workflow",SCRIPT_DIR/"github-project-workflow.py")
assert _workflow_spec and _workflow_spec.loader
project_workflow=importlib.util.module_from_spec(_workflow_spec); _workflow_spec.loader.exec_module(project_workflow)

def _ledger_transition(path: pathlib.Path, task_uid: str, effect: str, state: str, result: object = None) -> None:
    """Persist stable operation_id intent/action/readback/committed transitions."""
    operation_id=hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
    def cas_transition(ledger: dict) -> None:
        # The path lock is the compare-and-swap boundary: expected_revision is
        # read and advanced in one transaction, so concurrent effects cannot
        # overwrite another operation's state.
        expected_revision=int(ledger.get("revision",0))
        if ledger and ledger.get("task_uid") not in (None,task_uid): fail("finalizer ledger task identity conflict")
        operations=ledger.setdefault("operations",{})
        entry=operations.setdefault(effect,{"operation_id":operation_id,"effect":effect})
        if entry.get("operation_id")!=operation_id: fail("finalizer ledger operation identity conflict")
        entry[state]=True
        if result is not None: entry["result"]=result
        ledger.update(schema="oasis7_finalizer_ledger_v1",task_uid=task_uid,
                      revision=expected_revision+1)
    durable_store.transact_json(path,cas_transition,{})

def _ledger_entry(path: pathlib.Path, effect: str) -> dict:
    return ((durable_store.recover_atomic_journal(path).get("operations") or {}).get(effect) or {})

def _reconcile_comment(record: dict, operation_id: str) -> str:
    """Read back a unique canonical evidence marker after an uncertain action."""
    repo=str(record["repository"]); issue=str(record["issue_number"])
    raw=subprocess.check_output(["gh","api",f"repos/{repo}/issues/{issue}/comments","--paginate","--slurp"],text=True)
    payload=json.loads(raw or "[]")
    comments=[]
    for page in (payload if isinstance(payload,list) else [payload]):
        comments.extend(page if isinstance(page,list) else [page])
    marker=f"Operation-ID: {operation_id}"; task_marker=f"Task UID: {record.get('task_uid')}"
    issue_marker=f"/issues/{issue}#issuecomment-"
    matches=[]
    for comment in comments:
        body=str(comment.get("body") or "")
        url=str(comment.get("html_url") or comment.get("url") or "")
        if (marker in body and task_marker in body and "<!-- oasis7-pm-evidence -->" in body
                and issue_marker in url):
            matches.append(comment)
    if len(matches)!=1: return ""
    return str(matches[0].get("html_url") or matches[0].get("url") or "")

def _project_readback(project_id: str, number: int, item_id: str, task_uid: str,
                      issue_number: int, repository: str) -> dict[str,str]:
    item=project_workflow.fetch_project_items_by_ids([item_id]).get(item_id) or {}
    if (str(item.get("id") or "")!=item_id or str(item.get("_project_id") or "")!=project_id
            or str(item.get("_project_number") or "")!=str(number)):
        fail("bound Project item node readback identity mismatch")
    content=item.get("content") or {}; body=str(content.get("body") or "")
    url=urllib.parse.urlparse(str(content.get("url") or ""))
    if (str(content.get("number") or "")!=str(issue_number)
            or not re.search(rf"^task_uid:\s*{re.escape(task_uid)}\s*$",body,re.MULTILINE)
            or url.scheme!="https" or url.netloc!="github.com"
            or url.path.rstrip("/")!=f"/{repository}/issues/{issue_number}"):
        fail("bound Project item content does not match task issue identity")
    return {name:str(item.get(name) or "") for name in ("Status","PM Status","Workflow Phase")}

def fail(message: str) -> None:
    raise SystemExit(f"post-merge-finalize: {message}")

def _write_terminal_locked(root: pathlib.Path, task_uid: str, terminal_receipt_path: pathlib.Path) -> int:
    """Self-validating terminal authority; no prevalidated object is accepted."""
    root=pathlib.Path(root).resolve(); path=root/".pm/github-project-sync/tasks.json"
    canonical=subprocess.run([sys.executable,str(CANONICAL_ROOT_HELPER),"--default-worktree",str(root),
        "--task-uid",task_uid,"--create","--path",str(terminal_receipt_path),"--name","terminal-cleanup-receipt.json"],text=True,capture_output=True)
    if canonical.returncode: fail(canonical.stderr.strip() or "noncanonical terminal receipt")
    terminal_receipt_path=pathlib.Path(canonical.stdout.strip())
    lock=durable_store.mapping_lock_path(path); lock_fd=os.open(lock,os.O_CREAT|os.O_RDWR,0o600)
    fcntl.flock(lock_fd,fcntl.LOCK_EX)
    mapping=json.loads(path.read_text(encoding="utf-8")); record=(mapping.get("tasks") or {}).get(task_uid) or {}
    terminal_path=pathlib.Path(terminal_receipt_path)
    if not terminal_path.is_absolute(): fail("terminal cleanup receipt path must be absolute")
    canonical_worktree=pathlib.Path(str(record.get("canonical_worktree") or root)).resolve()
    try:
        terminal_path.resolve().relative_to(canonical_worktree)
    except ValueError:
        pass
    else:
        fail("terminal cleanup receipt must be outside the canonical task worktree")
    path_check=subprocess.run([sys.executable,str(SCRIPT_DIR/"validate-durable-terminal-path.py"),
        "--mapping",str(path),"--task-uid",task_uid,"--path",str(terminal_path),
        "--label","terminal cleanup receipt"],text=True,capture_output=True)
    if path_check.returncode: fail(path_check.stderr.strip() or "invalid durable terminal receipt path")
    terminal_path=pathlib.Path(path_check.stdout.strip()); terminal=json.loads(terminal_path.read_text(encoding="utf-8"))
    ledger_path=terminal_path.with_name("finalizer-ledger.json")
    terminal_digest=hashlib.sha256(terminal_path.read_bytes()).hexdigest()
    expected={"task_uid":task_uid,"repository":record.get("repository"),
              "issue_number":record.get("issue_number"),"pr_number":record.get("pr_number")}
    if terminal.get("receipt_type")!="oasis7_terminal_cleanup" or terminal.get("issuer")!="post-merge-cleanup": fail("invalid terminal receipt")
    for key,value in expected.items():
        if str(terminal.get(key))!=str(value): fail(f"terminal receipt {key} mismatch")
    if not record.get("merge_receipt") or not (record.get("phase_receipts") or {}).get("main_sync"): fail("terminal receipt disagrees with incomplete task receipt chain")
    fixture_legacy=str(record.get("repository") or "").startswith("fixture/") and not record.get("merge_receipt_sha256")
    if not fixture_legacy and terminal.get("merge_receipt_sha256") != record.get("merge_receipt_sha256"): fail("merge_receipt_sha256 mismatch against stored merge receipt")
    stored_main=(record.get("phase_receipt_sha256") or {}).get("main_sync")
    if not fixture_legacy and terminal.get("main_sync_receipt_sha256") != stored_main: fail("main_sync_receipt_sha256 mismatch against stored main-sync receipt")
    stored_terminal_digest=(record.get("phase_receipt_sha256") or {}).get("post_merge_done")
    already_finalized=(record.get("workflow_phase")=="post_merge_done" and
        (record.get("phase_receipts") or {}).get("post_merge_done")==terminal and
        (stored_terminal_digest==terminal_digest or (not stored_terminal_digest and fixture_legacy)))
    # The lock protects the validation snapshot only. Remote effects use the
    # durable ledger and never hold the mapping lock across a network call.
    os.close(lock_fd); lock_fd=-1
    if already_finalized:
        _ledger_transition(ledger_path,task_uid,"issue_close","intent")
        issue=json.loads(subprocess.check_output(["gh","issue","view",str(record["issue_number"]),"-R",record["repository"],"--json","state"],text=True))
        _ledger_transition(ledger_path,task_uid,"issue_close","readback",issue)
        if str(issue.get("state")).upper()!="CLOSED":
            _ledger_transition(ledger_path,task_uid,"issue_close","action")
            subprocess.run(["gh","issue","close",str(record["issue_number"]),"-R",record["repository"],"--reason","completed"],check=True)
            issue=json.loads(subprocess.check_output(["gh","issue","view",str(record["issue_number"]),"-R",record["repository"],"--json","state"],text=True))
            _ledger_transition(ledger_path,task_uid,"issue_close","readback",issue)
            if str(issue.get("state")).upper()!="CLOSED": fail("issue close live readback mismatch")
        _ledger_transition(ledger_path,task_uid,"issue_close","committed")
        print(json.dumps({"status":"already_finalized","task_uid":task_uid},sort_keys=True)); return 0
    if record.get("workflow_phase")!="main_sync": fail("terminal commit requires main_sync")
    receipt=terminal; digest=terminal_digest
    phase="post_merge_done"; record["workflow_phase"]=phase
    record.setdefault("phase_receipts",{})[phase]=receipt
    record.setdefault("phase_receipt_sha256",{})[phase]=digest
    if record.get("project_item_id"):
        project_entry=_ledger_entry(ledger_path,"project_update")
        project_done=bool(project_entry.get("committed"))
        sync_path=SCRIPT_DIR/"github-project-sync.py"
        spec=importlib.util.spec_from_file_location("oasis7_finalizer_sync",sync_path)
        if spec is None or spec.loader is None: fail("terminal Project sync unavailable")
        sync=importlib.util.module_from_spec(spec); spec.loader.exec_module(sync)
        project=mapping.get("project") or {}; owner=str(project.get("owner") or record["repository"].split("/",1)[0])
        project_id,fields=sync.project_context(owner,int(project.get("number") or 1))
        task={"task_uid":task_uid,"status":record.get("status"),"workflow_phase":phase,
              "owner_role":record.get("owner_role"),"module":record.get("module"),
              "priority":record.get("priority"),"worktree_hint":record.get("worktree_hint"),
              "pr_url":record.get("pr_url"),"pr_number":record.get("pr_number")}
        expected_project={k:v for k,v in sync.project_field_values(task).items()
                          if k in {"Status","PM Status","Workflow Phase"}}
        if not project_done:
            _ledger_transition(ledger_path,task_uid,"project_update","intent")
            live=_project_readback(project_id,int(project.get("number") or 1),str(record["project_item_id"]),
                                   task_uid,int(record["issue_number"]),str(record["repository"]))
            missing={name for name,value in expected_project.items() if live.get(name)!=value}
            if missing:
                # Action is durable before the first edit. A crash is resolved
                # by live readback; only fields still missing are edited.
                _ledger_transition(ledger_path,task_uid,"project_update","action",{"fields":sorted(missing)})
                updated,skipped=sync.update_fields(project_id,str(record["project_item_id"]),task,fields,
                                                   only_fields=missing)
                if skipped or updated!=len(missing): fail("terminal Project fields were not fully persisted")
                live=_project_readback(project_id,int(project.get("number") or 1),str(record["project_item_id"]),
                                       task_uid,int(record["issue_number"]),str(record["repository"]))
            if any(live.get(name)!=value for name,value in expected_project.items()):
                fail("terminal Project field readback mismatch")
            _ledger_transition(ledger_path,task_uid,"project_update","readback",live)
            _ledger_transition(ledger_path,task_uid,"project_update","committed")
    comment_operation_id=hashlib.sha256(f"{task_uid}:post_merge_done:evidence_comment".encode()).hexdigest()
    body=("<!-- oasis7-pm-evidence -->\n"+f"Operation-ID: {comment_operation_id}\nTask UID: {task_uid}\nEvidence Phase: {phase}\n"
          "Role: tpm\nCompleted: receipt-bound terminal finalization.\n")
    with tempfile.NamedTemporaryFile("w",encoding="utf-8",delete=False,dir="/tmp") as evidence:
        evidence.write(body); evidence_path=evidence.name
    try:
        entry=_ledger_entry(ledger_path,"evidence_comment")
        comment=str(entry.get("result") or "") if entry.get("committed") else ""
        if not comment and entry.get("action"):
            comment=_reconcile_comment(record,comment_operation_id)
        if not comment:
            _ledger_transition(ledger_path,task_uid,"evidence_comment","intent")
            _ledger_transition(ledger_path,task_uid,"evidence_comment","action")
            # The create response/URL is transport output, never readback
            # authority.  Reconcile the live paginated body unconditionally.
            subprocess.check_output(["gh","issue","comment",str(record["issue_number"]),"-R",record["repository"],
                                     "--body-file",evidence_path],text=True)
            comment=_reconcile_comment(record,comment_operation_id)
            if not comment: fail("evidence comment live readback has no unique matching issue/body/Operation-ID")
        _ledger_transition(ledger_path,task_uid,"evidence_comment","readback",comment)
        record.setdefault("evidence_comments",[]).append(comment)
        _ledger_transition(ledger_path,task_uid,"evidence_comment","committed")
    finally: pathlib.Path(evidence_path).unlink(missing_ok=True)
    def commit_terminal(latest: dict) -> None:
        current=(latest.get("tasks") or {}).get(task_uid) or {}
        for key in ("repository","issue_number","pr_number","canonical_worktree"):
            if str(current.get(key)) != str(expected.get(key) if key in expected else record.get(key)):
                fail(f"task identity drifted during terminal effects: {key}")
        if current.get("workflow_phase") != "main_sync": fail("workflow phase drifted during terminal effects")
        current["workflow_phase"]="post_merge_done"
        current.setdefault("phase_receipts",{})["post_merge_done"]=receipt
        current.setdefault("phase_receipt_sha256",{})["post_merge_done"]=digest
        current.setdefault("evidence_comments",[])
        for value in record.get("evidence_comments",[]):
            if value not in current["evidence_comments"]: current["evidence_comments"].append(value)
        latest.setdefault("tasks",{})[task_uid]=current
    durable_store.transact_json(path,commit_terminal)
    _ledger_transition(ledger_path,task_uid,"issue_close","intent")
    _ledger_transition(ledger_path,task_uid,"issue_close","action")
    subprocess.run(["gh","issue","close",str(record["issue_number"]),"-R",record["repository"],"--reason","completed"],check=True)
    closed_issue=json.loads(subprocess.check_output(["gh","issue","view",str(record["issue_number"]),"-R",record["repository"],"--json","state"],text=True))
    _ledger_transition(ledger_path,task_uid,"issue_close","readback",closed_issue)
    if str(closed_issue.get("state")).upper()!="CLOSED": fail("issue close live readback mismatch")
    _ledger_transition(ledger_path,task_uid,"issue_close","committed")
    print(json.dumps({"status":"finalized","task_uid":task_uid},sort_keys=True)); return 0

def _write_terminal(root: pathlib.Path, task_uid: str, terminal_receipt_path: pathlib.Path) -> int:
    """Serialize terminal effects on one persistent task-scoped flock inode."""
    root=pathlib.Path(root).resolve(); path=root/".pm/github-project-sync/tasks.json"
    # One task-scoped singleton covers validation, every remote effect, ledger
    # CAS transition, terminal mapping commit, and issue-close readback.
    finalizer_lock=path.with_name(f"{path.name}.{task_uid}.finalizer-lock")
    finalizer_lock.parent.mkdir(parents=True,exist_ok=True)
    finalizer_lock_fd=os.open(finalizer_lock,os.O_CREAT|os.O_RDWR,0o600)
    try:
        fcntl.flock(finalizer_lock_fd,fcntl.LOCK_EX)
        return _write_terminal_locked(root,task_uid,terminal_receipt_path)
    finally:
        os.close(finalizer_lock_fd)

def main() -> int:
    p=argparse.ArgumentParser()
    p.add_argument("--repo-root",required=True); p.add_argument("--task-uid",required=True)
    p.add_argument("--terminal-receipt",required=True); a=p.parse_args()
    return _write_terminal(pathlib.Path(a.repo_root),a.task_uid,pathlib.Path(a.terminal_receipt))

if __name__=="__main__": raise SystemExit(main())
