#!/usr/bin/env python3
"""Finalize one receipt-proven terminal workflow transition idempotently."""
from __future__ import annotations
import argparse, datetime as dt, hashlib, importlib.util, json, os, pathlib, re, subprocess, sys, tempfile, urllib.parse
from portable_file_lock import ensure_lock_byte, fcntl
from loop_terminal import (read_comments, read_issue, read_project, read_pull_request,
                           read_live_project_item, normalize_project_fields)
from task_complete_claim import select_historical_task_complete_claim
from terminal_proof import (DELIVERY_RECEIPT_FIELDS, read_live_repository, read_terminal_proof,
                            RECOVERY_RECEIPT_FIELDS, RECOVERY_TYPE, RECOVERY_MARKER,
                            receipt_chain_digest, terminal_delivery_comment_body,
                            validate_live_repository)

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
CANONICAL_ROOT_HELPER = SCRIPT_DIR/"canonical-receipt-root.py"
_store_spec=importlib.util.spec_from_file_location("workflow_durable_store",SCRIPT_DIR/"workflow-durable-store.py")
assert _store_spec and _store_spec.loader
durable_store=importlib.util.module_from_spec(_store_spec); _store_spec.loader.exec_module(durable_store)
_workflow_spec=importlib.util.spec_from_file_location("github_project_workflow",SCRIPT_DIR/"github-project-workflow.py")
assert _workflow_spec and _workflow_spec.loader
project_workflow=importlib.util.module_from_spec(_workflow_spec); _workflow_spec.loader.exec_module(project_workflow)

def _ledger_entry(path: pathlib.Path, effect: str) -> dict:
    return ((durable_store.recover_atomic_journal(path).get("operations") or {}).get(effect) or {})

def _reconcile_comment(record: dict, operation_id: str, expected_body: str) -> str:
    """Read back a unique canonical evidence marker after an uncertain action."""
    repo=str(record["repository"]); issue=str(record["issue_number"])
    raw=subprocess.check_output(["gh","api",f"repos/{repo}/issues/{issue}/comments","--paginate","--slurp"],text=True)
    payload=json.loads(raw or "[]")
    comments=[]
    for page in (payload if isinstance(payload,list) else [payload]):
        comments.extend(page if isinstance(page,list) else [page])
    marker=f"Operation-ID: {operation_id}"; task_marker=f"Task UID: {record.get('task_uid')}"
    issue_marker=f"/issues/{issue}#issuecomment-"
    matches=[]; legacy=[]
    for comment in comments:
        body=str(comment.get("body") or "")
        url=str(comment.get("html_url") or comment.get("url") or "")
        if (marker in body and task_marker in body and "<!-- oasis7-pm-evidence -->" in body
                and issue_marker in url):
            if body == expected_body:
                matches.append(comment)
            else:
                legacy.append(comment)
    if legacy:
        fail("existing terminal evidence comment lacks receipt-chain fields; replace it before resuming finalizer")
    if len(matches)!=1: return ""
    return str(matches[0].get("html_url") or matches[0].get("url") or "")

def _project_readback(project_id: str, number: int, item_id: str, task_uid: str,
                      issue_number: int, repository: str) -> dict[str,str]:
    item=read_live_project_item(repository,issue_number)
    context=item.get('project') or {}
    if (item.get("id")!=item_id or context.get('id')!=project_id
            or type(context.get('number')) is not int or context['number']!=number):
        fail("bound Project item node readback identity mismatch")
    content=item.get("content") or {}; body=str(content.get("body") or "")
    url=urllib.parse.urlparse(str(content.get("url") or ""))
    if (str(content.get("number") or "")!=str(issue_number)
            or not re.search(rf"^task_uid:\s*{re.escape(task_uid)}\s*$",body,re.MULTILINE)
            or url.scheme!="https" or url.netloc!="github.com"
            or url.path.rstrip("/")!=f"/{repository}/issues/{issue_number}"):
        fail("bound Project item content does not match task issue identity")
    fields=normalize_project_fields(item,repository)
    return {name:fields.get(name,"") for name in ("Status","PM Status","Workflow Phase")}

def fail(message: str) -> None:
    raise SystemExit(f"post-merge-finalize: {message}")

def _validate_cleanup_intent(terminal_path: pathlib.Path, task_uid: str,
                             record: dict, terminal: dict,
                             already_finalized: bool) -> None:
    """Do not finalize task truth while remote-branch cleanup is blocked."""
    intent_path=terminal_path.with_name("cleanup-intent.json")
    if not intent_path.exists():
        if terminal.get("cleanup_intent_required") is True:
            fail("current-protocol terminal receipt requires cleanup intent")
        if "cleanup_intent_required" in terminal:
            fail("terminal receipt cleanup intent marker is malformed")
        if already_finalized:
            return
        fail("legacy terminal receipt without cleanup intent requires an already-finalized task")
    try:
        intent=json.loads(intent_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cleanup intent is unreadable: {exc}")
    if not isinstance(intent,dict):
        fail("cleanup intent is malformed")
    expected={
        "receipt_type":"oasis7_cleanup_intent",
        "task_uid":task_uid,
        "repository":record.get("repository"),
        "branch":terminal.get("branch"),
    }
    for key,value in expected.items():
        if intent.get(key)!=value:
            fail(f"cleanup intent {key} identity mismatch")
    intent_worktree=intent.get("worktree")
    receipt_worktree=terminal.get("worktree")
    if not intent_worktree or not receipt_worktree or pathlib.Path(str(intent_worktree)).expanduser().resolve()!=pathlib.Path(str(receipt_worktree)).expanduser().resolve():
        fail("cleanup intent worktree identity mismatch")
    for flag in ("worktree_removed", "branch_deleted", "terminal_receipt_committed"):
        if intent.get(flag) is not True:
            fail(f"cleanup intent progress is incomplete: {flag}")
    blocker=intent.get("remote_branch_blocker")
    if blocker is None:
        return
    branch_tip=intent.get("branch_tip")
    if not isinstance(blocker,dict) or any(blocker.get(key)!=value for key,value in {
        "schema":"oasis7_cleanup_blocker_v1",
        "kind":"remote_branch_tip_mismatch",
        "branch":terminal.get("branch"),
        "expected_tip":branch_tip,
    }.items()):
        fail("cleanup intent remote branch blocker identity mismatch")
    if not isinstance(branch_tip,str) or not branch_tip or not isinstance(blocker.get("observed_tip"),str) or not blocker.get("observed_tip"):
        fail("cleanup intent remote branch blocker tip is malformed")
    if blocker.get("resolved") is not True:
        fail("cleanup intent has unresolved durable blocker: remote task branch tip disagrees with merged PR head")
    resolution=blocker.get("resolution")
    if (resolution not in {"matching_tip_deleted","remote_ref_absent"}
            or not blocker.get("resolved_at")
            or (resolution=="matching_tip_deleted" and blocker.get("resolved_tip")!=branch_tip)
            or (resolution=="remote_ref_absent" and blocker.get("resolved_tip")!="")):
        fail("cleanup intent remote branch blocker resolution is malformed")

def _write_terminal(root: pathlib.Path, task_uid: str, terminal_receipt_path: pathlib.Path) -> int:
    """Validate and serialize all v1 terminal effects from durable task truth."""
    root=pathlib.Path(root).resolve()
    if not re.fullmatch(r"task_[0-9a-f]{32}",task_uid): fail("invalid Task UID for terminal finalizer lock")
    path=root/".pm/github-project-sync/tasks.json"
    finalizer_lock=path.with_name(f"{path.name}.{task_uid}.finalizer-lock")
    finalizer_lock.parent.mkdir(parents=True,exist_ok=True)
    with finalizer_lock.open("a+b") as finalizer_lock_handle:
        ensure_lock_byte(finalizer_lock_handle)
        fcntl.flock(finalizer_lock_handle.fileno(),fcntl.LOCK_EX)

        # Mutations are closures over the task identity and receipt state
        # resolved below while this persistent task lock is held.
        def ledger_transition(ledger_path: pathlib.Path, effect: str, state: str,
                             result: object = None) -> None:
            operation_id=hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
            def cas_transition(ledger: dict) -> None:
                expected_revision=int(ledger.get("revision",0))
                if ledger and ledger.get("task_uid") not in (None,task_uid): fail("finalizer ledger task identity conflict")
                operations=ledger.setdefault("operations",{})
                entry=operations.setdefault(effect,{"operation_id":operation_id,"effect":effect})
                if entry.get("operation_id")!=operation_id: fail("finalizer ledger operation identity conflict")
                entry[state]=True
                if result is not None: entry["result"]=result
                ledger.update(schema="oasis7_finalizer_ledger_v1",task_uid=task_uid,
                              revision=expected_revision+1)
            durable_store.transact_json(ledger_path,cas_transition,{})

        def ensure_terminal_project(mapping: dict, record: dict, ledger_path: pathlib.Path) -> None:
            if not record.get("project_item_id"):
                return
            sync_path=SCRIPT_DIR/"github-project-sync.py"
            spec=importlib.util.spec_from_file_location("oasis7_finalizer_sync",sync_path)
            if spec is None or spec.loader is None: fail("terminal Project sync unavailable")
            sync=importlib.util.module_from_spec(spec); spec.loader.exec_module(sync)
            project=mapping.get("project") or {}; owner=str(project.get("owner") or record["repository"].split("/",1)[0])
            project_id,fields=sync.project_context(owner,int(project.get("number") or 1))
            task={"task_uid":task_uid,"status":record.get("status"),"workflow_phase":"post_merge_done",
                  "owner_role":record.get("owner_role"),"module":record.get("module"),
                  "priority":record.get("priority"),"worktree_hint":record.get("worktree_hint"),
                  "pr_url":record.get("pr_url"),"pr_number":record.get("pr_number")}
            expected_fields={k:v for k,v in sync.project_field_values(task).items()
                             if k in {"Status","PM Status","Workflow Phase"}}
            ledger_transition(ledger_path,"project_update","intent")
            live=_project_readback(project_id,int(project.get("number") or 1),str(record["project_item_id"]),
                                   task_uid,int(record["issue_number"]),str(record["repository"]))
            missing={name for name,value in expected_fields.items() if live.get(name)!=value}
            if missing:
                ledger_transition(ledger_path,"project_update","action",{"fields":sorted(missing)})
                updated,skipped=sync.update_fields(project_id,str(record["project_item_id"]),task,fields,
                                                   only_fields=missing)
                if skipped or updated!=len(missing): fail("terminal Project fields were not fully persisted")
                live=_project_readback(project_id,int(project.get("number") or 1),str(record["project_item_id"]),
                                       task_uid,int(record["issue_number"]),str(record["repository"]))
            if any(live.get(name)!=value for name,value in expected_fields.items()):
                fail("terminal Project field readback mismatch")
            ledger_transition(ledger_path,"project_update","readback",live)
            ledger_transition(ledger_path,"project_update","committed")

        def tombstone_value(record: dict, terminal_digest: str) -> dict:
            return {"schema":"oasis7_terminal_tombstone_v1","task_uid":record.get("task_uid"),
                "repository":record.get("repository"),"issue_number":record.get("issue_number"),
                "pr_number":record.get("pr_number"),"canonical_worktree":record.get("canonical_worktree"),
                "task_branch":record.get("task_branch"),"workflow_phase":"post_merge_done",
                "terminal_receipt_sha256":terminal_digest,"checkout_recreation_forbidden":True}

        def existing_tombstone_matches(tombstone_path: pathlib.Path, expected: dict) -> bool:
            try:
                raw=tombstone_path.read_bytes()
            except FileNotFoundError:
                return False
            except OSError as exc:
                fail(f"terminal tombstone is unreadable: {exc}")
            try:
                current=json.loads(raw)
            except (UnicodeDecodeError,json.JSONDecodeError) as exc:
                fail(f"terminal tombstone is malformed: {exc}")
            if not isinstance(current,dict):
                fail("terminal tombstone is malformed")
            for key,value in expected.items():
                actual=current.get(key)
                if type(actual) is not type(value) or actual!=value:
                    fail(f"terminal tombstone identity or receipt link mismatch: {key}")
            return True

        def write_tombstone(terminal_path: pathlib.Path, record: dict, terminal_digest: str) -> pathlib.Path:
            tombstone_path=terminal_path.with_name("terminal-tombstone.json")
            tombstone=tombstone_value(record,terminal_digest)
            if not existing_tombstone_matches(tombstone_path,tombstone):
                durable_store.replace_json(tombstone_path,tombstone)
            return tombstone_path

        def terminal_comment(record: dict, terminal: dict, terminal_digest: str) -> tuple[str,str]:
            phase="post_merge_done"
            operation_id=hashlib.sha256(f"{task_uid}:post_merge_done:evidence_comment".encode()).hexdigest()
            merge_receipt_digest=terminal.get("merge_receipt_sha256") or record.get("merge_receipt_sha256") or ""
            main_sync_receipt_digest=terminal.get("main_sync_receipt_sha256") or (record.get("phase_receipt_sha256") or {}).get("main_sync") or ""
            body=("<!-- oasis7-pm-evidence -->\n"+f"Operation-ID: {operation_id}\nTask UID: {task_uid}\nEvidence Phase: {phase}\n"
                  "Receipt Chain Version: 1\nReceipt Type: oasis7_terminal_cleanup\nReceipt Issuer: post-merge-cleanup\n"
                  f"PR Number: {record.get('pr_number')}\nPR URL: {record.get('pr_url')}\n"
                  f"Merge Receipt SHA256: {merge_receipt_digest}\n"
                  f"Main Sync Receipt SHA256: {main_sync_receipt_digest}\n"
                  f"Terminal Receipt SHA256: {terminal_digest}\n"
                  f"Receipt Chain Digest: {receipt_chain_digest(task_uid,record.get('repository'),record.get('issue_number'),record.get('pr_number'),record.get('pr_url'),merge_receipt_digest,main_sync_receipt_digest,terminal_digest)}\n"
                  "Role: tpm\nCompleted: receipt-bound terminal finalization.\n")
            return operation_id,body

        # Canonicalize the producer-selected file only after acquiring the
        # singleton task lock; the helper creates no alternate receipt path.
        canonical=subprocess.run([sys.executable,str(CANONICAL_ROOT_HELPER),"--default-worktree",str(root),
            "--task-uid",task_uid,"--create","--path",str(terminal_receipt_path),"--name","terminal-cleanup-receipt.json"],text=True,capture_output=True)
        if canonical.returncode: fail(canonical.stderr.strip() or "noncanonical terminal receipt")
        terminal_receipt_path=pathlib.Path(canonical.stdout.strip())
        mapping_lock=durable_store.mapping_lock_path(path)
        mapping_lock_handle=mapping_lock.open("a+b")
        ensure_lock_byte(mapping_lock_handle)
        fcntl.flock(mapping_lock_handle.fileno(),fcntl.LOCK_EX)
        try:
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
            recorded_worktree=pathlib.Path(str(record.get("canonical_worktree") or "")).expanduser().resolve()
            receipt_worktree=terminal.get("worktree")
            if not receipt_worktree: fail("terminal receipt worktree identity missing")
            if pathlib.Path(str(receipt_worktree)).expanduser().resolve()!=recorded_worktree:
                fail("terminal receipt worktree identity mismatch")
            if not terminal.get("branch"): fail("terminal receipt branch identity missing")
            if str(terminal.get("branch"))!=str(record.get("task_branch") or ""):
                fail("terminal receipt branch identity mismatch")
            if not record.get("merge_receipt") or not (record.get("phase_receipts") or {}).get("main_sync"):
                fail("terminal receipt disagrees with incomplete task receipt chain")
            fixture_legacy=str(record.get("repository") or "").startswith("fixture/") and not record.get("merge_receipt_sha256")
            if not fixture_legacy and terminal.get("merge_receipt_sha256")!=record.get("merge_receipt_sha256"):
                fail("merge_receipt_sha256 mismatch against stored merge receipt")
            stored_main=(record.get("phase_receipt_sha256") or {}).get("main_sync")
            if not fixture_legacy and terminal.get("main_sync_receipt_sha256")!=stored_main:
                fail("main_sync_receipt_sha256 mismatch against stored main-sync receipt")
            stored_terminal_digest=(record.get("phase_receipt_sha256") or {}).get("post_merge_done")
            already_finalized=(record.get("workflow_phase")=="post_merge_done" and
                (record.get("phase_receipts") or {}).get("post_merge_done")==terminal and
                (stored_terminal_digest==terminal_digest or (not stored_terminal_digest and fixture_legacy)))
            # A valid v1 tombstone can be pinned by an immutable aggregate-v1
            # child projection. Preserve its exact bytes, while rejecting a
            # stale or malformed link before any reconciliation effects. A
            # missing tombstone remains recoverable and is canonically created
            # after the existing terminal effects complete.
            existing_tombstone_matches(
                terminal_path.with_name("terminal-tombstone.json"),
                tombstone_value(record,terminal_digest))
            _validate_cleanup_intent(terminal_path,task_uid,record,terminal,already_finalized)
        finally:
            fcntl.flock(mapping_lock_handle.fileno(),fcntl.LOCK_UN)
            mapping_lock_handle.close()

        comment_operation_id,body=terminal_comment(record,terminal,terminal_digest)
        if already_finalized:
            entry=_ledger_entry(ledger_path,"evidence_comment")
            comment=_reconcile_comment(record,comment_operation_id,body)
            if not comment:
                fail("already-finalized terminal evidence comment has no unique live readback")
            if ((entry.get("committed") or entry.get("result"))
                    and str(entry.get("result") or "")!=comment):
                fail("already-finalized terminal evidence comment conflicts with live ledger binding")
            ensure_terminal_project(mapping,record,ledger_path)
            ledger_transition(ledger_path,"issue_close","intent")
            issue=json.loads(subprocess.check_output(["gh","issue","view",str(record["issue_number"]),"-R",record["repository"],"--json","state"],text=True))
            ledger_transition(ledger_path,"issue_close","readback",issue)
            if str(issue.get("state")).upper()!="CLOSED":
                ledger_transition(ledger_path,"issue_close","action")
                subprocess.run(["gh","issue","close",str(record["issue_number"]),"-R",record["repository"],"--reason","completed"],check=True)
                issue=json.loads(subprocess.check_output(["gh","issue","view",str(record["issue_number"]),"-R",record["repository"],"--json","state"],text=True))
                ledger_transition(ledger_path,"issue_close","readback",issue)
                if str(issue.get("state")).upper()!="CLOSED": fail("issue close live readback mismatch")
            ledger_transition(ledger_path,"issue_close","committed")
            write_tombstone(terminal_path,record,terminal_digest)
            print(json.dumps({"status":"already_finalized","task_uid":task_uid},sort_keys=True)); return 0
        if record.get("workflow_phase")!="main_sync": fail("terminal commit requires main_sync")
        receipt=terminal; digest=terminal_digest
        phase="post_merge_done"; record["workflow_phase"]=phase
        record.setdefault("phase_receipts",{})[phase]=receipt
        record.setdefault("phase_receipt_sha256",{})[phase]=digest
        ensure_terminal_project(mapping,record,ledger_path)
        with tempfile.NamedTemporaryFile("w",encoding="utf-8",delete=False,dir="/tmp") as evidence:
            evidence.write(body); evidence_path=evidence.name
        try:
            entry=_ledger_entry(ledger_path,"evidence_comment")
            if entry.get("committed"):
                live_comment=_reconcile_comment(record,comment_operation_id,body)
                recorded_comment=str(entry.get("result") or "")
                if not live_comment or recorded_comment!=live_comment:
                    fail("committed terminal evidence comment conflicts with unique live readback")
                comment=live_comment
            elif entry.get("action"):
                live_comment=_reconcile_comment(record,comment_operation_id,body)
                if not live_comment:
                    fail("terminal evidence comment action is uncertain and has no exact live readback")
                comment=live_comment
            else:
                ledger_transition(ledger_path,"evidence_comment","intent")
                ledger_transition(ledger_path,"evidence_comment","action")
                subprocess.check_output(["gh","issue","comment",str(record["issue_number"]),"-R",record["repository"],
                                         "--body-file",evidence_path],text=True)
                comment=_reconcile_comment(record,comment_operation_id,body)
                if not comment: fail("evidence comment live readback has no unique matching issue/body/Operation-ID")
            ledger_transition(ledger_path,"evidence_comment","readback",comment)
            record.setdefault("evidence_comments",[]).append(comment)
            ledger_transition(ledger_path,"evidence_comment","committed")
        finally:
            pathlib.Path(evidence_path).unlink(missing_ok=True)
        def commit_terminal(latest: dict) -> None:
            current=(latest.get("tasks") or {}).get(task_uid) or {}
            for key in ("repository","issue_number","pr_number","canonical_worktree"):
                if str(current.get(key))!=str(expected.get(key) if key in expected else record.get(key)):
                    fail(f"task identity drifted during terminal effects: {key}")
            if current.get("workflow_phase")!="main_sync": fail("workflow phase drifted during terminal effects")
            current["workflow_phase"]="post_merge_done"
            current.setdefault("phase_receipts",{})["post_merge_done"]=receipt
            current.setdefault("phase_receipt_sha256",{})["post_merge_done"]=digest
            current.setdefault("evidence_comments",[])
            for value in record.get("evidence_comments",[]):
                if value not in current["evidence_comments"]: current["evidence_comments"].append(value)
            latest.setdefault("tasks",{})[task_uid]=current
        durable_store.transact_json(path,commit_terminal)
        ledger_transition(ledger_path,"issue_close","intent")
        ledger_transition(ledger_path,"issue_close","action")
        subprocess.run(["gh","issue","close",str(record["issue_number"]),"-R",record["repository"],"--reason","completed"],check=True)
        closed_issue=json.loads(subprocess.check_output(["gh","issue","view",str(record["issue_number"]),"-R",record["repository"],"--json","state"],text=True))
        ledger_transition(ledger_path,"issue_close","readback",closed_issue)
        if str(closed_issue.get("state")).upper()!="CLOSED": fail("issue close live readback mismatch")
        ledger_transition(ledger_path,"issue_close","committed")
        write_tombstone(terminal_path,record,terminal_digest)
        print(json.dumps({"status":"finalized","task_uid":task_uid},sort_keys=True)); return 0


def _delivery_receipt_root(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    command=[sys.executable,str(CANONICAL_ROOT_HELPER),"--default-worktree",str(root),
             "--task-uid",task_uid,"--json"]
    try:
        observation = sys.modules.get("recovery_observation")
        raw=(observation.capture(command) if observation is not None and observation.active() is not None
             else subprocess.check_output(command,text=True,stderr=subprocess.PIPE))
        payload=json.loads(raw)
        return pathlib.Path(payload["receipt_root"])
    except (OSError,subprocess.SubprocessError,KeyError,TypeError,json.JSONDecodeError) as exc:
        raise ValueError("canonical delivery receipt root unavailable") from exc


def _load_json_object(path: pathlib.Path, label: str) -> tuple[bytes,dict]:
    raw=path.read_bytes()
    def unique(pairs):
        result={}
        for key,value in pairs:
            if key in result: raise ValueError(f"{label} contains duplicate JSON keys")
            result[key]=value
        return result
    value=json.loads(raw.decode("utf-8"),object_pairs_hook=unique)
    if not isinstance(value,dict): raise ValueError(f"{label} is not a JSON object")
    return raw,value


def _delivery_project_item(repository: str, issue_number: int, task_uid: str,
                           record: dict) -> tuple[dict,dict]:
    project=read_project(repository,issue_number)
    if (not project.get("id") or project.get("owner")!=repository.split("/",1)[0]
            or project.get("number")!=1 or project.get("page_complete") is not True):
        raise ValueError("canonical terminal Project identity/pagination unavailable")
    issue_url=f"https://github.com/{repository}/issues/{issue_number}"
    matches=[]
    for item in project.get("items",[]):
        context=item.get("project") or {}; content=item.get("content") or {}
        if (context.get("id")==project["id"] and content.get("number")==issue_number
                and content.get("url")==issue_url
                and re.findall(r"^task_uid:\s*([^\n]+)$",str(content.get("body") or "").replace("\r\n","\n"),re.MULTILINE)==[task_uid]):
            matches.append(item)
    if len(matches)!=1 or str(matches[0].get("id") or "")!=str(record.get("project_item_id") or ""):
        raise ValueError("bound Project task item is missing or ambiguous")
    return project,matches[0]


def _delivery_issue_binding(issue: dict, repository: str, task_uid: str,
                            issue_number: int, pr_number: int, pr_url: str,
                            *, terminal: bool, recovery: bool = False) -> None:
    issue_url=f"https://github.com/{repository}/issues/{issue_number}"
    body=str(issue.get("body") or "").replace("\r\n","\n")
    if (issue.get("number")!=issue_number or issue.get("html_url",issue.get("url"))!=issue_url
            or re.findall(r"^task_uid:\s*([^\n]+)$",body,re.MULTILINE)!=[task_uid]):
        raise ValueError("terminal delivery live Issue identity mismatch")
    for field,value in (("pr_number",str(pr_number)),("pr_url",pr_url)):
        matches=re.findall(rf"^- {re.escape(field)}: `([^`]+)`$",body,re.MULTILINE)
        if matches!=[value]: raise ValueError("terminal delivery live Issue PR binding mismatch")
    state=str(issue.get("state") or "").upper()
    reason=str(issue.get("state_reason",issue.get("stateReason","")) or "").lower()
    if terminal:
        if state!="CLOSED" or reason!="completed":
            raise ValueError("terminal delivery Issue is not closed as completed")
    elif recovery:
        if state not in {"OPEN", "CLOSED"} or (state=="CLOSED" and reason!="completed"):
            raise ValueError("terminal delivery Issue recovery state is not open or completed")
    elif state!="OPEN":
        raise ValueError("task_done delivery preflight requires the live Issue to be open")


def _delivery_pr_binding(pr: dict, repository: str, task_uid: str,
                         issue_number: int, pr_number: int, pr_url: str) -> tuple[str,str,str]:
    body=str(pr.get("body") or "").replace("\r\n","\n")
    base=pr.get("base") or {}; head=pr.get("head") or {}
    if (pr.get("number")!=pr_number or pr.get("html_url")!=pr_url
            or ((base.get("repo") or {}).get("full_name"))!=repository
            or ((head.get("repo") or {}).get("full_name"))!=repository):
        raise ValueError("terminal delivery PR reciprocal identity mismatch")
    if (re.findall(r"^Task: [^\n]+$",body,re.MULTILINE)!=[f"Task: {task_uid}"]
            or re.findall(r"^Refs #[1-9][0-9]*$",body,re.MULTILINE)!=[f"Refs #{issue_number}"]):
        raise ValueError("terminal delivery PR Task/Issue binding mismatch")
    if (str(pr.get("state") or "").upper()!="CLOSED" or pr.get("merged") is not True
            or not pr.get("merged_at")):
        raise ValueError("terminal delivery PR is not verified merged")
    head_oid=str(head.get("sha") or ""); merge_oid=str(pr.get("merge_commit_sha") or "")
    if not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})",head_oid) or not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})",merge_oid):
        raise ValueError("terminal delivery PR source or merge OID is invalid")
    return head_oid,merge_oid,str(base.get("ref") or "")


def _delivery_mapping_receipt(record: dict, root: pathlib.Path,
                              repository: str, pr_number: int, pr_url: str,
                              head_oid: str, merge_oid: str,
                              default_branch: str) -> tuple[bytes,dict,str]:
    path=root/"merge-receipt.json"
    try: raw,receipt=_load_json_object(path,"canonical merge receipt")
    except (OSError,UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
        raise ValueError(f"canonical merge receipt is unavailable or invalid: {exc}") from exc
    digest=hashlib.sha256(raw).hexdigest()
    expected={"receipt_type":"oasis7_pr_merge","issuer":"github_live_query",
              "evidence_mode":"production","repository":repository,
              "pr_number":pr_number,"pr_url":pr_url,"state":"MERGED",
              "head_oid":head_oid,"default_branch":default_branch,"base_ref":default_branch}
    if any(receipt.get(key)!=value for key,value in expected.items()) or not receipt.get("merged_at") or not receipt.get("observed_at"):
        raise ValueError("canonical merge receipt disagrees with live merged PR")
    if receipt.get("merge_commit_oid") not in (None,merge_oid):
        raise ValueError("canonical merge receipt merge commit disagrees with live merged PR")
    if record.get("merge_receipt")!=receipt or record.get("merge_receipt_sha256")!=digest:
        raise ValueError("task mapping merge receipt does not bind canonical receipt bytes")
    return raw,receipt,digest


def _delivery_live_context(root: pathlib.Path, task_uid: str, *,
                           prospective_readiness: dict | None = None) -> dict:
    _, mapping = _load_json_object(pathlib.Path(root)/".pm/github-project-sync/tasks.json", "canonical task mapping")
    record = (mapping.get("tasks") or {}).get(task_uid) or {}
    receipt_root = _delivery_receipt_root(pathlib.Path(root), task_uid)
    if ((record.get("phase_receipt_type") or {}).get("post_merge_done") == RECOVERY_TYPE
            or (receipt_root/"terminal-recovery-proof.json").exists()):
        import recovery_observation as observation
        with observation.observation():
            return _delivery_live_context_impl(root,task_uid,prospective_readiness=prospective_readiness)
    return _delivery_live_context_impl(root,task_uid,prospective_readiness=prospective_readiness)


def _delivery_live_context_impl(root: pathlib.Path, task_uid: str, *,
                           prospective_readiness: dict | None = None) -> dict:
    return _delivery_live_bindings(root, task_uid, prospective_readiness=prospective_readiness)


def _stored_delivery_live_context(root: pathlib.Path, task_uid: str) -> dict:
    """Internal existing-publication admission, never current readiness."""
    from terminal_recovery import _read_stored_recovery
    stored = _read_stored_recovery(root, task_uid)
    return _delivery_live_bindings(root, task_uid, stored_recovery=stored)


def _validate_delivery_project_entrance(fields: dict, record: dict,
                                       receipt_root: pathlib.Path, task_uid: str) -> None:
    terminal = {"Status": "Done", "PM Status": "done", "Workflow Phase": "done"}
    if any(fields.get(key) != terminal[key] for key in ("PM Status", "Workflow Phase")):
        raise ValueError("task_done Project projection is incomplete")
    phase = record.get("workflow_phase")
    if phase == "post_merge_done":
        if fields.get("Status") != "Done":
            raise ValueError("terminal Project projection is incomplete")
        return
    if phase not in {"task_done", "main_sync"}:
        raise ValueError("delivery finalization phase is invalid")
    if fields.get("Status") == "In Progress":
        return
    if fields.get("Status") != "Done":
        raise ValueError("task_done Project Status is not the producer prestate")
    # A persisted Project write may precede its response or later mapping
    # update. Admit that actual poststate only through the original journal.
    try:
        _, ledger = _load_json_object(receipt_root/"finalizer-ledger.json", "finalizer Project journal")
    except (OSError, UnicodeError, ValueError) as exc:
        raise ValueError("task_done Project Done lacks a valid finalizer journal") from exc
    operations = ledger.get("operations")
    entry = operations.get("project_update") if isinstance(operations, dict) else None
    operation_id = hashlib.sha256(f"{task_uid}:post_merge_done:project_update".encode()).hexdigest()
    if (ledger.get("schema") != "oasis7_finalizer_ledger_v1" or ledger.get("task_uid") != task_uid
            or type(ledger.get("revision")) is not int or ledger["revision"] <= 0
            or not isinstance(entry, dict) or entry.get("operation_id") != operation_id
            or entry.get("effect") != "project_update" or entry.get("intent") is not True
            or any(key in entry and type(entry[key]) is not bool for key in ("action", "readback", "committed"))):
        raise ValueError("task_done Project Done finalizer journal identity mismatch")
    result = entry.get("result")
    if entry.get("readback") is True:
        if result != terminal:
            raise ValueError("task_done Project Done journal readback mismatch")
    elif entry.get("action") is True and entry.get("committed") is not True:
        if result != {"fields": ["Status"]}:
            raise ValueError("task_done Project Done journal action mismatch")
    else:
        raise ValueError("task_done Project Done lacks a recorded finalizer action/readback")


def _delivery_live_bindings(root: pathlib.Path, task_uid: str, *,
                            prospective_readiness: dict | None = None,
                            stored_recovery: dict | None = None) -> dict:
    root=pathlib.Path(root).resolve()
    mapping_path=root/".pm/github-project-sync/tasks.json"
    # Read-only admission must not reconcile a journal before readiness is
    # known. Unreadable/uncertain mapping stops before terminal effects.
    _,mapping=_load_json_object(mapping_path,"canonical task mapping")
    record=(mapping.get("tasks") or {}).get(task_uid) or {}
    if not isinstance(record,dict) or record.get("task_uid") not in (None,task_uid):
        raise ValueError("canonical task mapping is missing or has a conflicting UID")
    if (record.get("status")!="done" or record.get("workflow_phase") not in {"task_done","main_sync","post_merge_done"}
            or record.get("completion_mode") not in (None,"pr_task","single_pr")):
        raise ValueError("delivery finalization requires the canonical single-PR task_done mapping")
    repository=str(record.get("repository") or "")
    issue_number=record.get("issue_number"); pr_number=record.get("pr_number")
    pr_url=str(record.get("pr_url") or "")
    if (not re.fullmatch(r"[^/\s]+/[^/\s]+",repository) or type(issue_number) is not int or issue_number<=0
            or type(pr_number) is not int or pr_number<=0
            or pr_url!=f"https://github.com/{repository}/pull/{pr_number}"):
        raise ValueError("canonical task/Issue/PR mapping identity is invalid")
    if not record.get("project_item_id") or not record.get("canonical_worktree") or not record.get("task_branch"):
        raise ValueError("canonical task Project/worktree/branch identity is incomplete")

    receipt_root=_delivery_receipt_root(root,task_uid)
    issue=read_issue(repository,issue_number)
    pr=read_pull_request(repository,pr_number)
    comments=read_comments(repository,issue_number)
    project,project_item=_delivery_project_item(repository,issue_number,task_uid,record)
    head_oid,merge_oid,base_ref=_delivery_pr_binding(pr,repository,task_uid,issue_number,pr_number,pr_url)
    existing_path=receipt_root/"terminal-delivery-receipt.json"
    existing=None
    if existing_path.exists():
        try: _,existing=_load_json_object(existing_path,"terminal delivery receipt")
        except (OSError,UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
            raise ValueError(f"existing terminal delivery receipt is invalid: {exc}") from exc
    prior_target=existing.get("observed_target_oid") if isinstance(existing,dict) else None
    live_repository=read_live_repository(repository,merge_oid,prior_target)
    repo_data=live_repository.get("repository") or {}
    ref=live_repository.get("ref") or {}
    default_branch=str(repo_data.get("default_branch") or "")
    target_oid=str((ref.get("object") or {}).get("sha") or "")
    if (not default_branch or base_ref!=default_branch
            or record.get("default_branch") not in (None,default_branch)):
        raise ValueError("terminal delivery live default branch disagrees with merged PR or task mapping")
    if not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})",target_oid):
        raise ValueError("terminal delivery live target OID is invalid")
    validate_live_repository(live_repository,repository,merge_oid,
                             default_branch=default_branch,
                             observed_target_oid=prior_target or target_oid)
    _,merge_receipt,merge_digest=_delivery_mapping_receipt(record,receipt_root,repository,
                                                            pr_number,pr_url,head_oid,merge_oid,default_branch)
    selector=(record.get("phase_receipt_type") or {}).get("post_merge_done")
    mapped_v1=(record.get("phase_receipts") or {}).get("post_merge_done",{}).get("receipt_type")=="oasis7_terminal_cleanup"
    _delivery_issue_binding(issue,repository,task_uid,issue_number,pr_number,pr_url,
                            terminal=(record.get("workflow_phase")=="post_merge_done" and mapped_v1),
                            recovery=(selector in ("oasis7_terminal_delivery", RECOVERY_TYPE)))
    fields=normalize_project_fields(project_item,repository)
    _validate_delivery_project_entrance(fields, record, receipt_root, task_uid)
    if record.get("workflow_phase")=="post_merge_done":
        if (record.get("phase_receipt_type") or {}).get("post_merge_done") in ("oasis7_terminal_delivery", RECOVERY_TYPE):
            pass
        elif (record.get("phase_receipts") or {}).get("post_merge_done",{}).get("receipt_type")=="oasis7_terminal_cleanup":
            # The caller will validate a complete v1 proof and return it without migration.
            pass
        else:
            raise ValueError("terminal task has an unknown or mixed post_merge_done protocol")
    elif record.get("workflow_phase") not in {"task_done","main_sync"}:
        raise ValueError("delivery finalization phase is invalid")
    legacy_v1 = (record.get("workflow_phase") == "post_merge_done"
                 and (record.get("phase_receipts") or {}).get("post_merge_done", {}).get("receipt_type") == "oasis7_terminal_cleanup")
    if legacy_v1:
        claim,claim_digest = None,None
        readiness = None
        recovery = None
    elif selector == RECOVERY_TYPE or (receipt_root/"terminal-recovery-proof.json").exists():
        if selector not in (None, RECOVERY_TYPE):
            raise ValueError("terminal recovery cannot replace a selected ordinary protocol")
        from terminal_recovery import validate_recovery
        recovery = (validate_recovery(root, task_uid) if stored_recovery is None
                    else stored_recovery)
        if recovery["receipt_root"] != receipt_root:
            raise ValueError("terminal recovery canonical receipt root mismatch")
        for key, value in {"repository":repository,"task_uid":task_uid,"issue_number":issue_number,
                           "pr_number":pr_number,"pr_url":pr_url,"head_oid":head_oid,
                           "merge_commit_oid":merge_oid,"default_branch":default_branch,
                           "observed_target_oid":(target_oid if stored_recovery is None
                                                   else recovery["proof"]["observed_target_oid"])}.items():
            if recovery["proof"].get(key) != value:
                raise ValueError(f"terminal recovery live binding mismatch: {key}")
        claim = recovery["completion"]
        claim_digest = "sha256:" + recovery["completion_digest"]
        readiness = None
    else:
        recovery = None
        from readiness_transport import validate_readiness_proof
        readiness = validate_readiness_proof(root, task_uid, record,
            live_pr=pr, comments=comments, live_issue=issue,
            proof=(prospective_readiness if not (receipt_root/"readiness-proof.json").exists() else None))
        claim,claim_digest,_claim_comment=select_historical_task_complete_claim(
            repository,task_uid,record,issue,comments,accepted_head=head_oid)
    return {"root":root,"mapping_path":mapping_path,"mapping":mapping,"record":record,
            "task_uid":task_uid,
            "receipt_root":receipt_root,"issue":issue,"project":project,"project_item":project_item,
            "pr":pr,"comments":comments,"live_repository":live_repository,
            "repository":repository,"issue_number":issue_number,"pr_number":pr_number,"pr_url":pr_url,
            "head_oid":head_oid,"merge_commit_oid":merge_oid,"default_branch":default_branch,
            "observed_target_oid":prior_target or target_oid,"target_oid":target_oid,
            "merge_receipt":merge_receipt,"merge_receipt_sha256":merge_digest,
            "claim":claim,"task_complete_claim_sha256":claim_digest,"existing_delivery":existing,
            "readiness_proof_sha256":readiness["digest"] if readiness else None,
            "recovery":recovery,"protocol_version":3 if recovery else 2}


def _validate_existing_delivery_record(existing: dict, expected: dict) -> None:
    fields = RECOVERY_RECEIPT_FIELDS if expected.get("receipt_type") == RECOVERY_TYPE else DELIVERY_RECEIPT_FIELDS
    if set(expected) != fields:
        raise ValueError("terminal delivery writer schema does not match the strict receipt schema")
    if set(existing) != fields:
        raise ValueError("terminal delivery receipt closed schema mismatch")
    observed_at=existing.get("observed_at")
    try:
        if not isinstance(observed_at,str):
            raise ValueError("timestamp must be a string")
        parsed=dt.datetime.fromisoformat(observed_at.replace("Z","+00:00"))
        if parsed.tzinfo is None:
            raise ValueError("timezone required")
    except (TypeError,ValueError) as exc:
        raise ValueError("terminal delivery receipt observation time is invalid") from exc
    for key,value in expected.items():
        if key=="observed_at":
            continue
        if type(existing[key]) is not type(value) or existing[key]!=value:
            raise ValueError(f"existing delivery receipt conflicts with current live {key}")

def _delivery_record(context: dict) -> dict:
    existing=context.get("existing_delivery")
    record={
        "receipt_type":"oasis7_terminal_delivery","schema_version":2,
        "issuer":"post-merge-finalize","evidence_mode":"production",
        "task_uid":context["task_uid"],
        "repository":context["repository"],"issue_number":context["issue_number"],
        "pr_number":context["pr_number"],"pr_url":context["pr_url"],
        "head_oid":context["head_oid"],"merge_commit_oid":context["merge_commit_oid"],
        "default_branch":context["default_branch"],"observed_target_oid":context["observed_target_oid"],
        "merge_receipt_sha256":context["merge_receipt_sha256"],
        "task_complete_claim_sha256":context["task_complete_claim_sha256"],
        "readiness_proof_sha256":context["readiness_proof_sha256"],
        "worktree":str(pathlib.Path(str(context["record"].get("canonical_worktree"))).expanduser().resolve()),
        "branch":str(context["record"].get("task_branch")),"completion_semantics":"delivery_only",
        "observed_at":dt.datetime.now(dt.timezone.utc).isoformat(),
    }
    if context.get("recovery") is not None:
        record.pop("readiness_proof_sha256")
        record.update(receipt_type=RECOVERY_TYPE, schema_version=1,
                      recovery_proof_sha256=context["recovery"]["digest"])
    if existing is not None:
        # A receipt written before a lost local/remote response is immutable.
        # Validate its closed schema, original observation time, and every
        # typed authority field before allowing any terminal side effect.
        # Keep the original timestamp and exact file bytes on valid recovery.
        _validate_existing_delivery_record(existing,record)
        record=existing
    return record

def _delivery_partial_legacy_effects(context: dict) -> None:
    """Do not migrate a v1 comment effect whose response may have been lost."""
    record=context["record"]
    if record.get("workflow_phase")=="post_merge_done":
        if (record.get("phase_receipt_type") or {}).get("post_merge_done") in ("oasis7_terminal_delivery", RECOVERY_TYPE):
            return
        if (record.get("phase_receipts") or {}).get("post_merge_done",{}).get("receipt_type")=="oasis7_terminal_cleanup":
            return
    ledger_path=context["receipt_root"]/"finalizer-ledger.json"
    v2_expected_body=None
    if context.get("existing_delivery") is not None:
        _,receipt_bytes=_load_json_object(context["receipt_root"]/"terminal-delivery-receipt.json","terminal delivery receipt")
        # The exact extant receipt is validated against current authorities
        # before its comment can be used as recovery evidence.
        expected_receipt=_delivery_record(context)
        if any(receipt_bytes.get(k)!=v for k,v in expected_receipt.items() if k!="observed_at"):
            raise ValueError("existing delivery receipt conflicts with the live accepted authority")
        raw=(context["receipt_root"]/"terminal-delivery-receipt.json").read_bytes()
        v2_expected_body=terminal_delivery_comment_body(receipt_bytes,hashlib.sha256(raw).hexdigest())
    ledger_exists=ledger_path.exists()
    if ledger_exists:
        try:
            _,ledger=_load_json_object(ledger_path,"finalizer ledger")
        except (OSError,UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
            raise ValueError(f"existing finalizer ledger is invalid: {exc}") from exc
        if ledger.get("task_uid") not in (None,context["task_uid"]):
            raise ValueError("existing finalizer ledger task identity mismatch")
        operations=ledger.get("operations") or {}
        effect=operations.get("evidence_comment") or {}
        issue_close=operations.get("issue_close") or {}
        if v2_expected_body is not None:
            comment=_delivery_comment_readback(context,v2_expected_body)
            if (effect.get("result") and not isinstance(effect.get("result"),dict)):
                raise ValueError("legacy finalizer comment result requires v1 reconciliation; v2 migration is blocked")
            if (effect.get("action") or effect.get("committed")) and comment is None:
                raise ValueError("v2 comment action is uncertain and no exact comment is visible; manual reconciliation is required")
            if comment is not None and not (effect.get("action") or effect.get("committed")):
                raise ValueError("v2 comment exists without a recorded finalizer action")
            if issue_close.get("action") or issue_close.get("committed"):
                raise ValueError("partial legacy Issue-close effect requires v1 reconciliation; v2 migration is blocked")
        elif effect.get("action") or effect.get("committed") or issue_close.get("action") or issue_close.get("committed"):
            raise ValueError("partial legacy finalizer effect requires v1 reconciliation; v2 migration is blocked")
    elif v2_expected_body is not None and _delivery_comment_readback(context,v2_expected_body) is not None:
        raise ValueError("v2 terminal evidence comment exists without a finalizer ledger action")
    operation_id=hashlib.sha256(f"{context['task_uid']}:post_merge_done:evidence_comment".encode()).hexdigest()
    for comment in context["comments"]:
        body=str(comment.get("body") or "")
        if ("<!-- oasis7-pm-evidence -->" in body
                and f"Operation-ID: {operation_id}" in body
                and f"Task UID: {context['task_uid']}" in body
                and "Evidence Phase: post_merge_done" in body):
            raise ValueError("legacy v1 terminal evidence comment exists without a selected v1 proof; reconcile it before v2 migration")


def _delivery_status(context: dict) -> dict:
    record=context["record"]
    version=context["protocol_version"]
    if record.get("workflow_phase")=="post_merge_done":
        selector=(record.get("phase_receipt_type") or {}).get("post_merge_done")
        if selector in ("oasis7_terminal_delivery", RECOVERY_TYPE):
            try:
                proof=read_terminal_proof(
                    context["root"],context["task_uid"],record,
                    live_issue=context["issue"],live_project_item=context["project_item"],
                    live_pr=context["pr"],live_repository=context["live_repository"],
                    comments=context["comments"],
                )
                if proof.get("status")!="passed" or proof.get("protocol_version")!=version:
                    raise ValueError("selected v2 terminal delivery proof is not complete")
            except ValueError:
                _validate_resumable_v2(context)
                return {"status":"ready","protocol_version":version,"task_uid":context["task_uid"],
                        "resume":True,"delivery":{"state":"pending","protocol_version":version}}
            return {"status":"already_finalized","protocol_version":proof["protocol_version"],
                    "task_uid":context["task_uid"],"proof":proof,
                    "delivery":{"state":"complete","protocol_version":version}}
        proof=read_terminal_proof(
            context["root"],context["task_uid"],record,
            live_issue=context["issue"],live_project_item=context["project_item"],
            live_pr=context["pr"],live_repository=context["live_repository"],
            comments=context["comments"],
        )
        return {"status":"already_finalized","protocol_version":proof["protocol_version"],
                "task_uid":context["task_uid"],"proof":proof}
    _delivery_partial_legacy_effects(context)
    return {"status":"ready","protocol_version":version,"task_uid":context["task_uid"],
            "delivery":{"state":"pending","protocol_version":version},
            "head_oid":context["head_oid"],"merge_commit_oid":context["merge_commit_oid"],
            "default_branch":context["default_branch"],
            "observed_target_oid":context["observed_target_oid"],
            "merge_receipt_sha256":context["merge_receipt_sha256"],
            "task_complete_claim_sha256":context["task_complete_claim_sha256"]}


def _validate_resumable_v2(context: dict) -> None:
    """Prove that a selected v2 protocol is at a recoverable local boundary."""
    record=context["record"]; task_uid=context["task_uid"]
    receipt_path=context["receipt_root"]/"terminal-delivery-receipt.json"
    try:
        raw,receipt=_load_json_object(receipt_path,"terminal delivery receipt")
    except (OSError,UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
        raise ValueError(f"selected v2 terminal delivery is not resumable: {exc}") from exc
    digest=hashlib.sha256(raw).hexdigest()
    expected_type = RECOVERY_TYPE if context.get("recovery") else "oasis7_terminal_delivery"
    fields = RECOVERY_RECEIPT_FIELDS if context.get("recovery") else DELIVERY_RECEIPT_FIELDS
    if (set(receipt)!=fields
            or (record.get("phase_receipt_type") or {}).get("post_merge_done")!=expected_type
            or (record.get("phase_receipt_sha256") or {}).get("post_merge_done")!=digest
            or (record.get("phase_receipts") or {}).get("post_merge_done")!=receipt):
        raise ValueError("selected v2 terminal delivery mapping/receipt is not resumable")
    expected=_delivery_record(context)
    if any(receipt.get(key)!=value for key,value in expected.items() if key!="observed_at"):
        raise ValueError("selected v2 terminal delivery authority changed before recovery")
    body=terminal_delivery_comment_body(receipt,digest)
    comment=_delivery_comment_readback(context,body)
    comment_digest=hashlib.sha256(body.encode("utf-8")).hexdigest()
    if (comment is None
            or (record.get("phase_receipt_comment_id") or {}).get("post_merge_done")!=comment.get("id")
            or (record.get("phase_receipt_comment_sha256") or {}).get("post_merge_done")!=comment_digest):
        raise ValueError("selected v2 terminal delivery comment is not resumable")
    ledger_path=context["receipt_root"]/"finalizer-ledger.json"
    try:
        _,ledger=_load_json_object(ledger_path,"finalizer ledger")
    except (OSError,UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
        raise ValueError(f"selected v2 finalizer ledger is not resumable: {exc}") from exc
    if ledger.get("schema")!="oasis7_finalizer_ledger_v1" or ledger.get("task_uid")!=task_uid:
        raise ValueError("selected v2 finalizer ledger identity is not resumable")
    operations=ledger.get("operations") or {}
    for effect in ("project_update","evidence_comment"):
        item=operations.get(effect) or {}
        op_id=hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
        if (item.get("operation_id")!=op_id or item.get("effect")!=effect or item.get("committed") is not True):
            raise ValueError(f"selected v2 finalizer {effect} is not resumable")
    project_result=(operations.get("project_update") or {}).get("result") or {}
    if any(project_result.get(k)!=v for k,v in {"Status":"Done","PM Status":"done","Workflow Phase":"done"}.items()):
        raise ValueError("selected v2 finalizer Project readback is not resumable")
    comment_result=(operations.get("evidence_comment") or {}).get("result") or {}
    if (comment_result.get("comment_id")!=comment.get("id")
            or comment_result.get("comment_sha256")!=comment_digest):
        raise ValueError("selected v2 finalizer comment readback is not resumable")
    issue_entry=operations.get("issue_close") or {}
    expected_issue_op=hashlib.sha256(f"{task_uid}:post_merge_done:issue_close".encode()).hexdigest()
    if issue_entry and (issue_entry.get("operation_id")!=expected_issue_op or issue_entry.get("effect")!="issue_close"):
        raise ValueError("selected v2 finalizer Issue close operation identity is not resumable")
    issue_state=str(context["issue"].get("state") or "").upper()
    issue_reason=str(context["issue"].get("state_reason",context["issue"].get("stateReason","")) or "").lower()
    if issue_state not in {"OPEN","CLOSED"} or (issue_state=="CLOSED" and issue_reason!="completed"):
        raise ValueError("selected v2 live Issue state is not resumable")
    if issue_entry.get("committed"):
        result=issue_entry.get("result") or {}
        if str(result.get("state") or "").upper()!="CLOSED" or str(result.get("state_reason",result.get("stateReason","")) or "").lower()!="completed":
            raise ValueError("selected v2 Issue close readback is not resumable")
    tombstone=context["receipt_root"]/"terminal-tombstone.json"
    if tombstone.exists():
        _,value=_load_json_object(tombstone,"terminal tombstone")
        expected_tombstone={"schema":"oasis7_terminal_tombstone_v1","task_uid":task_uid,
            "repository":context["repository"],"issue_number":context["issue_number"],
            "pr_number":context["pr_number"],"canonical_worktree":receipt["worktree"],
            "task_branch":receipt["branch"],"workflow_phase":"post_merge_done",
            "terminal_receipt_sha256":digest,"checkout_recreation_forbidden":True}
        if value!=expected_tombstone:
            raise ValueError("selected v2 terminal tombstone conflicts with receipt")


def validate_prior_delivery_admission(root: pathlib.Path, task_uid: str,
                                      readiness: dict) -> None:
    """Reuse authoritative prior-delivery admission before any proof writes.

    The prospective readiness record is revalidated in memory. No missing
    proof file is manufactured to let the existing receipt/recovery parser run.
    First delivery without prior terminal evidence stays outside this prior-only
    path, including its pre-TaskDone completion-claim boundary.
    """
    root=pathlib.Path(root).resolve()
    _,mapping=_load_json_object(root/".pm/github-project-sync/tasks.json","canonical task mapping")
    record=(mapping.get("tasks") or {}).get(task_uid) or {}
    receipt_root=_delivery_receipt_root(root,task_uid)
    phase="post_merge_done"
    selected=any(isinstance(record.get(key),dict) and phase in record[key]
                 for key in ("phase_receipt_type","phase_receipt_sha256","phase_receipts",
                             "phase_receipt_comment_id","phase_receipt_comment_sha256"))
    prior=any((receipt_root/name).exists() for name in
              ("terminal-delivery-receipt.json","finalizer-ledger.json","terminal-tombstone.json"))
    if not selected and not prior:
        return
    context=_delivery_live_context(root,task_uid,prospective_readiness=readiness)
    _delivery_status(context)


def _validate_existing_terminal_namespace(root: pathlib.Path, task_uid: str) -> None:
    """Read-only existing namespace admission, not fresh delivery acceptance."""
    root = pathlib.Path(root).resolve()
    _, mapping = _load_json_object(root/".pm/github-project-sync/tasks.json", "canonical task mapping")
    record = (mapping.get("tasks") or {}).get(task_uid)
    if not isinstance(record, dict):
        raise ValueError("terminal namespace canonical task mapping missing")
    receipt_root = _delivery_receipt_root(root, task_uid)
    selected = any(isinstance(record.get(key), dict) and "post_merge_done" in record[key]
                   for key in ("phase_receipt_type", "phase_receipt_sha256", "phase_receipts",
                               "phase_receipt_comment_id", "phase_receipt_comment_sha256"))
    filenames = ("terminal-delivery-receipt.json", "terminal-cleanup-receipt.json",
                 "finalizer-ledger.json", "terminal-tombstone.json")
    present = [name for name in filenames if (receipt_root/name).exists()]
    for name in present:
        _, candidate = _load_json_object(receipt_root/name, "existing terminal namespace "+name)
        expected = {
            "terminal-delivery-receipt.json": ("receipt_type", {"oasis7_terminal_delivery", RECOVERY_TYPE}),
            "terminal-cleanup-receipt.json": ("receipt_type", {"oasis7_terminal_cleanup"}),
            "finalizer-ledger.json": ("schema", {"oasis7_finalizer_ledger_v1"}),
            "terminal-tombstone.json": ("schema", {"oasis7_terminal_tombstone_v1"}),
        }
        key, types = expected[name]
        if candidate.get(key) not in types:
            raise ValueError("existing terminal namespace unknown typed record: "+name)
    if not selected and not present:
        return
    selector = (record.get("phase_receipt_type") or {}).get("post_merge_done")
    mapped_v1 = (record.get("phase_receipts") or {}).get("post_merge_done", {}).get("receipt_type") == "oasis7_terminal_cleanup"
    if "terminal-cleanup-receipt.json" in present and not mapped_v1:
        raise ValueError("existing terminal cleanup requires its unchanged v1 selector")
    recovery = selector == RECOVERY_TYPE or (receipt_root/"terminal-recovery-proof.json").exists()
    context = (_stored_delivery_live_context(root, task_uid) if recovery
               else _delivery_live_context(root, task_uid))
    if not recovery:
        _delivery_status(context)
        return
    if selector not in (None, RECOVERY_TYPE):
        raise ValueError("existing terminal namespace mixed protocol")
    if record.get("workflow_phase") != "post_merge_done":
        _delivery_partial_legacy_effects(context)
        if (receipt_root/"terminal-tombstone.json").exists():
            raise ValueError("existing terminal tombstone without selected terminal protocol")
        return
    if selector != RECOVERY_TYPE:
        raise ValueError("existing terminal namespace missing recovery selector")
    from terminal_proof import _read_v2_files, _validate_existing_recovery_terminal
    try:
        files = _read_v2_files(root, task_uid)
        receipt = files["delivery"]["record"]
        digest = files["delivery"]["digest"]
        if ((record.get("phase_receipt_sha256") or {}).get("post_merge_done") != digest
                or (record.get("phase_receipts") or {}).get("post_merge_done") != receipt):
            raise ValueError("existing terminal namespace receipt selector mismatch")
        _validate_existing_recovery_terminal(receipt, files, task_uid, record,
            context["issue"], context["project_item"], context["pr"],
            context["live_repository"], context["comments"])
    except ValueError:
        # Same supported selected partial boundary as the effect consumer;
        # no collector/readiness call and no journal reconciliation writes.
        _validate_resumable_v2(context)


def _delivery_comment_readback(context: dict, body: str) -> dict | None:
    issue_url=f"https://github.com/{context['repository']}/issues/{context['issue_number']}"
    marker=RECOVERY_MARKER if context.get("recovery") else "<!-- oasis7-pm-evidence/v2 -->"
    markers=[]; exact=[]
    for comment in context["comments"]:
        comment_body=str(comment.get("body") or "")
        if marker in comment_body:
            markers.append(comment)
        author=comment.get("user") or {}
        if comment_body==body and (not isinstance(author,dict)
                or author.get("login")!=str(context["repository"]).split("/",1)[0]):
            raise ValueError("terminal v2 evidence comment author mismatch")
        if (comment_body==body and type(comment.get("id")) is int and comment["id"]>0
                and comment.get("html_url")==f"{issue_url}#issuecomment-{comment['id']}"):
            exact.append(comment)
    if len(markers)>1 or (markers and (len(exact)!=1 or markers[0] is not exact[0])):
        raise ValueError("terminal delivery comment readback mismatch: marker is duplicate or has a noncanonical body")
    if len(exact)>1:
        raise ValueError("terminal delivery comment readback mismatch: exact body is duplicated")
    return exact[0] if exact else None


def _write_delivery(root: pathlib.Path, task_uid: str) -> dict:
    root=pathlib.Path(root).resolve()
    if not re.fullmatch(r"task_[0-9a-f]{32}",task_uid):
        raise ValueError("invalid Task UID for delivery finalizer lock")
    mapping_path=root/".pm/github-project-sync/tasks.json"
    lock=mapping_path.with_name(f"{mapping_path.name}.{task_uid}.finalizer-lock")
    lock.parent.mkdir(parents=True,exist_ok=True)
    with lock.open("a+b") as handle:
        ensure_lock_byte(handle)
        fcntl.flock(handle.fileno(),fcntl.LOCK_EX)
        # Every mutation below is a closure over the lock-held canonical task
        # context; importing this module exposes no context-taking v2 writer.
        context=_delivery_live_context(root,task_uid)
        state=_delivery_status(context)
        if state["status"]=="already_finalized":
            print(json.dumps({k:v for k,v in state.items() if k!="proof"},sort_keys=True)); return state
        _delivery_partial_legacy_effects(context)

        create_root=[sys.executable,str(CANONICAL_ROOT_HELPER),"--default-worktree",str(root),
                     "--task-uid",task_uid,"--json","--create"]
        try:
            root_payload=json.loads(subprocess.check_output(create_root,text=True,stderr=subprocess.PIPE))
            context["receipt_root"]=pathlib.Path(root_payload["receipt_root"])
        except (OSError,subprocess.SubprocessError,KeyError,TypeError,json.JSONDecodeError) as exc:
            raise ValueError("canonical delivery receipt root unavailable") from exc

        def ledger_transition(effect: str, transition: str, result: object = None) -> None:
            operation_id=hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
            def cas_transition(ledger: dict) -> None:
                expected_revision=int(ledger.get("revision",0))
                if ledger and ledger.get("task_uid") not in (None,task_uid):
                    raise ValueError("finalizer ledger task identity conflict")
                operations=ledger.setdefault("operations",{})
                entry=operations.setdefault(effect,{"operation_id":operation_id,"effect":effect})
                if entry.get("operation_id")!=operation_id:
                    raise ValueError("finalizer ledger operation identity conflict")
                entry[transition]=True
                if result is not None: entry["result"]=result
                ledger.update(schema="oasis7_finalizer_ledger_v1",task_uid=task_uid,
                              revision=expected_revision+1)
            durable_store.transact_json(ledger_path,cas_transition,{})

        def ensure_project_updated() -> None:
            mapping=context["mapping"]
            record=context["record"]
            sync_path=SCRIPT_DIR/"github-project-sync.py"
            spec=importlib.util.spec_from_file_location("oasis7_finalizer_sync",sync_path)
            if spec is None or spec.loader is None:
                raise ValueError("terminal Project sync unavailable")
            sync=importlib.util.module_from_spec(spec); spec.loader.exec_module(sync)
            project=mapping.get("project") or {}
            owner=str(project.get("owner") or record["repository"].split("/",1)[0])
            project_id,fields=sync.project_context(owner,int(project.get("number") or 1))
            task={"task_uid":task_uid,"status":record.get("status"),"workflow_phase":"post_merge_done",
                  "owner_role":record.get("owner_role"),"module":record.get("module"),
                  "priority":record.get("priority"),"worktree_hint":record.get("worktree_hint"),
                  "pr_url":record.get("pr_url"),"pr_number":record.get("pr_number")}
            expected={k:v for k,v in sync.project_field_values(task).items()
                      if k in {"Status","PM Status","Workflow Phase"}}
            ledger_transition("project_update","intent")
            live=_project_readback(project_id,int(project.get("number") or 1),
                str(record["project_item_id"]),task_uid,int(record["issue_number"]),str(record["repository"]))
            missing={name for name,value in expected.items() if live.get(name)!=value}
            if missing:
                ledger_transition("project_update","action",{"fields":sorted(missing)})
                updated,skipped=sync.update_fields(project_id,str(record["project_item_id"]),task,fields,
                                                   only_fields=missing)
                if skipped or updated!=len(missing):
                    raise ValueError("terminal Project fields were not fully persisted")
                live=_project_readback(project_id,int(project.get("number") or 1),
                    str(record["project_item_id"]),task_uid,int(record["issue_number"]),str(record["repository"]))
            if any(live.get(name)!=value for name,value in expected.items()):
                raise ValueError("terminal Project field readback mismatch")
            ledger_transition("project_update","readback",live)
            ledger_transition("project_update","committed")

        def create_comment(body: str) -> dict:
            entry=_ledger_entry(ledger_path,"evidence_comment")
            comment=_delivery_comment_readback(context,body)
            if entry.get("committed"):
                result=entry.get("result") or {}
                if (comment is None or result.get("comment_id")!=comment.get("id")
                        or result.get("comment_sha256")!=hashlib.sha256(body.encode("utf-8")).hexdigest()):
                    raise ValueError("terminal delivery finalizer ledger comment binding conflicts with live readback")
                return comment
            if entry.get("action"):
                if comment is None:
                    raise ValueError("terminal delivery comment action is uncertain and no exact comment is visible; manual reconciliation is required")
                return comment
            if comment is not None:
                raise ValueError("terminal delivery comment exists without a recorded finalizer action")
            ledger_transition("evidence_comment","intent")
            ledger_transition("evidence_comment","action")
            with tempfile.NamedTemporaryFile("w",encoding="utf-8",newline="",delete=False,dir="/tmp") as evidence:
                evidence.write(body); evidence_path=evidence.name
            try:
                subprocess.check_output(["gh","issue","comment",str(context["issue_number"]),
                    "-R",context["repository"],"--body-file",evidence_path],text=True)
            finally:
                pathlib.Path(evidence_path).unlink(missing_ok=True)
            context["comments"]=read_comments(context["repository"],context["issue_number"])
            comment=_delivery_comment_readback(context,body)
            if comment is None:
                raise ValueError("terminal delivery comment write has no unique live readback")
            return comment

        def commit_mapping(receipt: dict, digest: str, comment: dict, comment_digest: str) -> None:
            def commit(latest: dict) -> None:
                current=(latest.get("tasks") or {}).get(task_uid)
                if not isinstance(current,dict):
                    raise ValueError("canonical task disappeared during terminal delivery")
                for key in ("repository","issue_number","pr_number","pr_url","canonical_worktree","task_branch","merge_receipt_sha256"):
                    if str(current.get(key))!=str(context["record"].get(key)):
                        raise ValueError(f"canonical task identity drifted during delivery: {key}")
                if current.get("workflow_phase")=="post_merge_done":
                    selected=(current.get("phase_receipt_type") or {}).get("post_merge_done")
                    if selected!=receipt["receipt_type"]:
                        raise ValueError("canonical task already selected another terminal protocol")
                    if ((current.get("phase_receipt_sha256") or {}).get("post_merge_done")!=digest
                            or (current.get("phase_receipts") or {}).get("post_merge_done")!=receipt):
                        raise ValueError("canonical task terminal delivery selector conflicts with receipt")
                elif current.get("workflow_phase") not in {"task_done","main_sync"}:
                    raise ValueError("canonical workflow phase drifted before delivery selector commit")
                current["workflow_phase"]="post_merge_done"
                current.setdefault("phase_receipts",{})["post_merge_done"]=receipt
                current.setdefault("phase_receipt_type",{})["post_merge_done"]=receipt["receipt_type"]
                current.setdefault("phase_receipt_sha256",{})["post_merge_done"]=digest
                current.setdefault("phase_receipt_comment_id",{})["post_merge_done"]=comment["id"]
                current.setdefault("phase_receipt_comment_sha256",{})["post_merge_done"]=comment_digest
                current.setdefault("evidence_comments",[])
                url=comment.get("html_url")
                if url and url not in current["evidence_comments"]:
                    current["evidence_comments"].append(url)
                latest.setdefault("tasks",{})[task_uid]=current
            durable_store.transact_json(context["mapping_path"],commit)

        def write_tombstone(receipt_digest: str) -> pathlib.Path:
            path=context["receipt_root"]/"terminal-tombstone.json"
            _,delivery=_load_json_object(context["receipt_root"]/"terminal-delivery-receipt.json","terminal delivery receipt")
            expected={"schema":"oasis7_terminal_tombstone_v1","task_uid":task_uid,
                "repository":context["repository"],"issue_number":context["issue_number"],
                "pr_number":context["pr_number"],"canonical_worktree":delivery["worktree"],
                "task_branch":delivery["branch"],"workflow_phase":"post_merge_done",
                "terminal_receipt_sha256":receipt_digest,"checkout_recreation_forbidden":True}
            if path.exists():
                _,existing=_load_json_object(path,"terminal tombstone")
                if existing!=expected:
                    raise ValueError("existing terminal delivery tombstone conflicts with receipt")
            else:
                durable_store.replace_json(path,expected)
            return path

        # Resolve all authority while holding the task lock before any write.
        if context.get("recovery") is not None:
            from terminal_recovery import validate_recovery
            fresh=validate_recovery(root,task_uid)
            if (fresh["digest"]!=context["recovery"]["digest"]
                    or fresh["completion_digest"]!=context["recovery"]["completion_digest"]):
                raise ValueError("terminal recovery authority changed before effects")
        receipt=_delivery_record(context)
        receipt_path=context["receipt_root"]/"terminal-delivery-receipt.json"
        if not receipt_path.exists():
            durable_store.replace_json(receipt_path,receipt)
        raw,stored_receipt=_load_json_object(receipt_path,"terminal delivery receipt")
        if stored_receipt!=receipt:
            raise ValueError("canonical terminal delivery receipt changed during finalization")
        receipt_digest=hashlib.sha256(raw).hexdigest()
        ledger_path=context["receipt_root"]/"finalizer-ledger.json"
        ensure_project_updated()
        context["comments"]=read_comments(context["repository"],context["issue_number"])
        body=terminal_delivery_comment_body(receipt,receipt_digest)
        comment=create_comment(body)
        comment_digest=hashlib.sha256(str(comment["body"]).encode("utf-8")).hexdigest()
        ledger_transition("evidence_comment","readback",
                          {"comment_id":comment["id"],"comment_sha256":comment_digest})
        ledger_transition("evidence_comment","committed")
        commit_mapping(receipt,receipt_digest,comment,comment_digest)

        issue=read_issue(context["repository"],context["issue_number"])
        ledger_transition("issue_close","intent")
        issue_state=str(issue.get("state") or "").upper()
        issue_reason=str(issue.get("state_reason",issue.get("stateReason","")) or "").lower()
        if issue_state=="OPEN":
            ledger_transition("issue_close","action")
            subprocess.run(["gh","issue","close",str(context["issue_number"]),"-R",context["repository"],"--reason","completed"],check=True)
            issue=read_issue(context["repository"],context["issue_number"])
            issue_state=str(issue.get("state") or "").upper()
            issue_reason=str(issue.get("state_reason",issue.get("stateReason","")) or "").lower()
        if issue_state!="CLOSED" or issue_reason!="completed":
            raise ValueError("terminal delivery Issue close readback mismatch")
        ledger_transition("issue_close","readback",issue)
        ledger_transition("issue_close","committed")
        write_tombstone(receipt_digest)

        latest=durable_store.recover_atomic_journal(context["mapping_path"])
        latest_record=(latest.get("tasks") or {}).get(task_uid)
        if not isinstance(latest_record,dict):
            raise ValueError("canonical task mapping disappeared after terminal delivery")
        _,project_item=_delivery_project_item(context["repository"],context["issue_number"],task_uid,latest_record)
        pr=read_pull_request(context["repository"],context["pr_number"])
        comments=read_comments(context["repository"],context["issue_number"])
        live_repository=read_live_repository(context["repository"],context["merge_commit_oid"],receipt["observed_target_oid"])
        proof=read_terminal_proof(root,task_uid,latest_record,live_issue=issue,
            live_project_item=project_item,live_pr=pr,live_repository=live_repository,comments=comments)
        if proof.get("protocol_version")!=context["protocol_version"] or proof.get("status")!="passed":
            raise ValueError("terminal delivery finalizer readback did not select v2 proof")
        result={"status":"finalized","protocol_version":context["protocol_version"],"task_uid":task_uid,
                "terminal_receipt_sha256":receipt_digest,"comment_id":comment["id"],
                "delivery":{"state":"complete","protocol_version":context["protocol_version"]}}
        print(json.dumps(result,sort_keys=True)); return result


def _preflight_delivery(root: pathlib.Path, task_uid: str) -> dict:
    context=_delivery_live_context(root,task_uid)
    state=_delivery_status(context)
    result={k:v for k,v in state.items() if k!="proof"}
    result["preflight"]=True
    print(json.dumps(result,sort_keys=True))
    return result


def main() -> int:
    p=argparse.ArgumentParser()
    p.add_argument("--repo-root",required=True); p.add_argument("--task-uid",required=True)
    p.add_argument("--terminal-receipt"); p.add_argument("--delivery",action="store_true")
    p.add_argument("--preflight",action="store_true"); p.add_argument("--json",action="store_true")
    a=p.parse_args()
    if a.delivery:
        if a.terminal_receipt:
            fail("--delivery cannot be combined with --terminal-receipt")
        try:
            _preflight_delivery(pathlib.Path(a.repo_root),a.task_uid) if a.preflight else _write_delivery(pathlib.Path(a.repo_root),a.task_uid)
        except (OSError,subprocess.SubprocessError,ValueError,KeyError,TypeError,json.JSONDecodeError) as exc:
            fail(str(exc))
        return 0
    if a.preflight or a.json or not a.terminal_receipt:
        p.error("legacy finalization requires --terminal-receipt; --preflight/--json require --delivery")
    return _write_terminal(pathlib.Path(a.repo_root),a.task_uid,pathlib.Path(a.terminal_receipt))

if __name__=="__main__": raise SystemExit(main())
