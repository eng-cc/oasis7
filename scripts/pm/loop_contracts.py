"""Live GitHub contract provenance, immutable contents and consumption gates.

Injected authority readers are test seams, never caller-authored live evidence.
No background revocation listener or automatic downstream task is created.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess

REPOSITORY = "eng-cc/oasis7"
SCHEMA = "oasis7.loop-contract/v1"
MARKER = "oasis7-loop-contract"
OID = re.compile(r"[0-9a-f]{40}\Z")
UID = re.compile(r"task_[0-9a-f]{32}\Z")


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()


def contract_digest(contract):
    # Eligibility is live authority, while all content/approval inputs freeze.
    return "sha256:" + hashlib.sha256(canonical({k:v for k,v in contract.items() if k != "eligibility"})).hexdigest()


def result(errors, **fields):
    return {"status":"blocked" if errors else "passed", "blockers":errors, **fields}


def git(root, *args):
    process = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if process.returncode:
        raise ValueError("contract Git content unavailable: " + process.stderr.decode("utf-8", "replace").strip())
    return process.stdout


def safe_path(path):
    return isinstance(path,str) and bool(path) and not path.startswith("/") and "\\" not in path and not any(ord(c)<32 for c in path) and not any(p in {"", ".", ".."} for p in path.split("/")) and not re.match(r"^[A-Za-z]:",path)


def ensure_contract_objects(root, contract):
    """Acquire only approved exact objects after PR identity validation."""
    missing = []
    for key in ('source_head', 'merged_head'):
        probe = subprocess.run(['git', '-C', str(root), 'cat-file', '-e', contract[key] + '^{commit}'], capture_output=True)
        if probe.returncode: missing.append(key)
    if not missing: return
    origin = git(root, 'config', '--get', 'remote.origin.url').decode().strip()
    if origin not in ('https://github.com/' + REPOSITORY + '.git', 'https://github.com/' + REPOSITORY, 'git@github.com:' + REPOSITORY + '.git', 'ssh://git@github.com/' + REPOSITORY + '.git'):
        raise ValueError('missing contract objects require canonical repository origin')
    for key in missing:
        oid = contract[key]
        if key == 'source_head':
            # GitHub retains PR refs after squash/source branch deletion. A
            # moved ref is merely acquisition, never authority for a newer SHA.
            subprocess.run(['git', '-C', str(root), 'fetch', '--no-write-fetch-head', '--no-tags', 'origin', f"refs/pull/{contract['approval_ref']['pr_number']}/head"], capture_output=True)
        if subprocess.run(['git', '-C', str(root), 'cat-file', '-e', oid + '^{commit}'], capture_output=True).returncode:
            fetched = subprocess.run(['git', '-C', str(root), 'fetch', '--no-write-fetch-head', '--no-tags', 'origin', oid], capture_output=True)
            if fetched.returncode:
                raise ValueError('approved exact contract object unavailable: ' + oid)
        if subprocess.run(['git', '-C', str(root), 'cat-file', '-e', oid + '^{commit}'], capture_output=True).returncode:
            raise ValueError('approved exact contract object unavailable after fetch: ' + oid)


def validate_contract_record(contract, root, pr):
    errors=[]
    if not isinstance(contract,dict) or contract.get("schema")!=SCHEMA:
        return ["invalid contract schema"]
    if not isinstance(contract.get("contract_id"),str) or not contract["contract_id"].strip() or type(contract.get("revision")) is not int or contract["revision"]<1:
        errors.append("invalid contract identity/revision")
    if contract.get("owner_loop") not in {"product","system"}:
        errors.append("contract owner must be product or system")
    approval=contract.get("approval_ref",{})
    if not isinstance(approval,dict) or approval.get("repository")!=REPOSITORY or type(approval.get("pr_number")) is not int or approval["pr_number"]<1:
        errors.append("invalid canonical PR approval reference")
    if not isinstance(pr,dict) or pr.get("merged") is not True or pr.get("number")!=approval.get("pr_number"):
        errors.append("approval PR is not verified merged")
    for key,pr_key in (("source_head","head"),("merged_head","merge_commit")):
        if not isinstance(contract.get(key),str) or not OID.fullmatch(contract[key]) or contract[key]!=pr.get(pr_key):
            errors.append(f"contract {key} does not match merged PR")
    if not errors:
        try:
            ensure_contract_objects(root, contract)
        except (ValueError, OSError) as exc:
            errors.append(str(exc))
    eligibility=contract.get("eligibility")
    if not isinstance(eligibility,dict) or any(type(eligibility.get(k)) is not bool for k in ("new_tasks","in_flight","release")):
        errors.append("explicit consumption eligibility required")
    if not isinstance(contract.get("scope"),list) or not contract["scope"] or any(not isinstance(s,str) or not s.strip() for s in contract["scope"]):
        errors.append("contract scope required")
    if not isinstance(contract.get("upstream_contracts"),list):
        errors.append("upstream contract list required")
    refs=contract.get("content_refs")
    if not isinstance(refs,list) or not refs:
        return errors+["contract content references required"]
    seen=set()
    for ref in refs:
        if not isinstance(ref,dict) or not safe_path(ref.get("path")) or ref["path"] in seen:
            errors.append("unsafe or duplicate content path")
            continue
        seen.add(ref["path"])
        if not isinstance(ref.get("clauses"),list) or not ref["clauses"] or any(not isinstance(c,str) or not c.strip() for c in ref["clauses"]) or len(set(ref["clauses"]))!=len(ref["clauses"]):
            errors.append("content clauses must be explicit unique identifiers")
        if not isinstance(ref.get("sha256"),str) or not re.fullmatch(r"sha256:[0-9a-f]{64}",ref["sha256"]):
            errors.append("invalid content digest")
            continue
        if errors:
            continue
        try:
            for key in ("source_head","merged_head"):
                entry=git(root,"ls-tree",contract[key],"--",ref["path"])
                if not entry.startswith(b"100644 blob "):
                    raise ValueError("contract content must be regular non-executable file")
                raw=git(root,"show",contract[key]+":"+ref["path"])
                if "sha256:"+hashlib.sha256(raw).hexdigest()!=ref["sha256"]:
                    errors.append("approved/published content mismatch: "+ref["path"])
        except (ValueError,OSError) as exc:
            errors.append(str(exc))
    return errors


class GitHubAuthority:
    """Production reader: server fields and current admin permission only."""
    def __init__(self, repo_root):
        self.repo_root=Path(repo_root)

    def api(self,path,body=None,paginate=False):
        command=["gh","api",path]
        if paginate:
            command += ["--paginate","--slurp"]
        if body is not None:
            command += ["--method","POST","--input","-"]
        process=subprocess.run(command,cwd=self.repo_root,input=canonical(body) if body is not None else None,capture_output=True)
        if process.returncode:
            raise ValueError("live GitHub authority read/write failed: "+process.stderr.decode("utf-8","replace").strip())
        return json.loads(process.stdout)

    def issue(self,number,task_uid=None):
        if type(number) is not int or number<1:
            raise ValueError("invalid publication issue")
        issue=self.api(f"repos/{REPOSITORY}/issues/{number}")
        body=(issue.get("body") or "").replace("\r\n","\n")
        fields=re.findall(r"(?m)^task_uid:[^\n]*$",body)
        matches=re.findall(r"(?m)^task_uid: (task_[0-9a-f]{32})$",body)
        if issue.get("number")!=number or "<!-- oasis7-pm-task -->" not in body or len(fields)!=1 or len(matches)!=1 or "pull_request" in issue:
            raise ValueError("publication issue task identity mismatch")
        if task_uid is not None and matches[0]!=task_uid:
            raise ValueError("publication task UID mismatch")
        return issue,matches[0]

    def permission(self,login):
        if not isinstance(login,str) or not re.fullmatch(r"[A-Za-z0-9-]+",login):
            raise ValueError("invalid server author")
        return self.api(f"repos/{REPOSITORY}/collaborators/{login}/permission").get("permission")

    def pr(self,number):
        if type(number) is not int or number<1:
            raise ValueError("invalid PR number")
        pr=self.api(f"repos/{REPOSITORY}/pulls/{number}")
        if (pr.get("base",{}).get("repo") or {}).get("full_name")!=REPOSITORY:
            raise ValueError("approval PR repository mismatch")
        return {"number":pr.get("number"),"merged":pr.get("merged"),"head":pr.get("head",{}).get("sha"),"merge_commit":pr.get("merge_commit_sha")}

    def __call__(self,reference):
        ref=reference.get("publication_ref",{})
        issue_number,comment_id=ref.get("issue_number"),ref.get("comment_id")
        _,uid=self.issue(issue_number)
        if type(comment_id) is not int or comment_id<1:
            raise ValueError("invalid publication comment identity")
        comment=self.api(f"repos/{REPOSITORY}/issues/comments/{comment_id}")
        if comment.get("id")!=comment_id or comment.get("issue_url")!=f"https://api.github.com/repos/{REPOSITORY}/issues/{issue_number}":
            raise ValueError("publication comment target mismatch")
        payload=json.loads(comment.get("body") or "")
        if not isinstance(payload,dict) or payload.get("marker")!=MARKER or payload.get("task_uid")!=uid or not isinstance(payload.get("contract"),dict):
            raise ValueError("publication marker/task mismatch")
        contract=payload["contract"]
        if payload.get("contract_digest")!=contract_digest(contract):
            raise ValueError("publication immutable contract digest mismatch")
        author=comment.get("user",{}).get("login")
        return {"repository":REPOSITORY,"issue_number":issue_number,"comment_id":comment_id,"task_uid":uid,"author":author,"permission":self.permission(author),"contract":contract,"pr":self.pr(contract.get("approval_ref",{}).get("pr_number"))}

    def obligation(self,obligation):
        from loop_terminal import validate_terminal_delivery
        checked = validate_terminal_delivery(REPOSITORY, obligation.get("task_uid"), obligation.get("issue_number"), repo_root=self.repo_root)
        return checked['status'] == 'passed'

    def publish(self,binding,contract,before_write=None):
        number=binding.get("issue_number",contract.get("publication_issue"))
        self.issue(number,binding.get("task_uid"))
        login=self.api("user").get("login")
        if self.permission(login)!="admin":
            raise ValueError("publication requires live repository admin authorization")
        payload={"marker":MARKER,"task_uid":binding["task_uid"],"contract_digest":contract_digest(contract),"contract":contract}
        # Exact logical action is reconciled before POST, including a retry
        # after a previous successful POST whose response was lost.
        pages=self.api(f"repos/{REPOSITORY}/issues/{number}/comments?per_page=100",paginate=True)
        matches=[]
        for page in pages:
            for comment in page:
                try:
                    previous=json.loads(comment.get("body") or "")
                except (ValueError,TypeError):
                    continue
                if not isinstance(previous,dict) or not isinstance(previous.get("contract"),dict):
                    continue
                prior=previous["contract"]
                if previous.get("marker")==MARKER and prior.get("contract_id")==contract["contract_id"] and prior.get("revision")==contract["revision"]:
                    if previous.get("task_uid")!=binding["task_uid"] or previous.get("contract_digest")!=payload["contract_digest"]:
                        raise ValueError("conflicting contract publication; reconcile_required")
                    matches.append(comment["id"])
        if len(matches)>1:
            raise ValueError("duplicate publication identity; reconcile_required")
        if matches:
            return {"issue_number":number,"comment_id":matches[0]}
        # All authority checks and duplicate/conflict reads above are read-only.
        # Persist intent only at the first potentially effectful request.
        if before_write is not None:
            before_write()
        created=self.api(f"repos/{REPOSITORY}/issues/{number}/comments",{"body":canonical(payload).decode()})
        if type(created.get("id")) is not int:
            raise ValueError("publication response uncertain; reconcile_required")
        return {"issue_number":number,"comment_id":created["id"]}


def validate_contracts(tool_root,target_repo_root,binding,authority_reader=None,purpose="in_flight"):
    errors=[]
    if purpose not in {"new_tasks","in_flight","release"}:
        return result(["unknown consumption purpose"])
    if not isinstance(binding,dict) or not isinstance(binding.get("input_contracts"),list):
        return result(["input_contracts list required"])
    reader=authority_reader or GitHubAuthority(target_repo_root)
    visited=set()
    active=set()
    revisions={}
    def inspect(reference):
        if not isinstance(reference,dict):
            errors.append("invalid contract reference")
            return
        if not isinstance(reference.get("contract_id"),str) or type(reference.get("revision")) is not int or reference["revision"]<1:
            errors.append("invalid contract reference identity")
            return
        key=(reference.get("contract_id"),reference.get("revision"))
        if key in active:
            errors.append("cyclic upstream contract")
            return
        # Every reference's consumed clauses/digest must be checked even when
        # its shared upstream revision was already visited by another input.
        try:
            record=reader(reference)
            ref=reference.get("publication_ref",{})
            if record.get("repository")!=REPOSITORY or record.get("issue_number")!=ref.get("issue_number") or record.get("comment_id")!=ref.get("comment_id") or record.get("permission")!="admin" or not record.get("author") or not UID.fullmatch(record.get("task_uid", "")):
                raise ValueError("publication provenance/admin authority mismatch")
            contract=record.get("contract",{})
            if (contract.get("contract_id"),contract.get("revision"))!=key:
                raise ValueError("contract immutable identity mismatch")
            if reference.get("contract_digest")!=contract_digest(contract):
                raise ValueError("bound contract digest mismatch")
            digest=reference["contract_digest"]
            if revisions.setdefault(key,digest)!=digest:
                raise ValueError("conflicting immutable contract revision")
            errors.extend(validate_contract_record(contract,target_repo_root,record.get("pr",{})))
            if contract.get("eligibility",{}).get(purpose) is not True:
                errors.append("contract withdrawn or ineligible for "+purpose)
            clauses=reference.get("consumed_clauses")
            allowed={c for item in contract.get("content_refs",[]) for c in item.get("clauses",[])}
            if not isinstance(clauses,list) or not clauses or any(not isinstance(c,str) or c not in allowed for c in clauses) or len(set(clauses))!=len(clauses):
                errors.append("unapproved or duplicate consumed clauses")
            if binding.get("target_delivery") not in contract.get("scope",[]):
                errors.append("contract does not cover target delivery")
            if key not in visited:
                active.add(key)
                for upstream in contract.get("upstream_contracts",[]):
                    inspect(upstream)
                active.remove(key)
                visited.add(key)
        except (ValueError,OSError,KeyError,TypeError,AttributeError) as exc:
            errors.append(str(exc))
    for reference in binding["input_contracts"]:
        inspect(reference)
    if purpose=="release":
        for obligation in binding.get("delivery_obligations",[]):
            try:
                if not hasattr(reader,"obligation") or not reader.obligation(obligation):
                    errors.append("delivery obligation lacks terminal live readback: "+str(obligation.get("id")))
            except (ValueError,OSError,KeyError,TypeError) as exc:
                errors.append(str(exc))
    return result(errors, purpose=purpose, contracts_checked=len(visited), authority="injected_test_reader" if authority_reader is not None else "live_github")


def publish_contract(tool_root,target_repo_root,binding,contract,authority_reader=None,before_write=None):
    reader=authority_reader or GitHubAuthority(target_repo_root)
    try:
        if not isinstance(binding,dict) or not UID.fullmatch(binding.get("task_uid","")) or not binding.get("manual_request_ref"):
            return result(["publication requires bound task/manual request context"])
        if binding.get("loop")!=contract.get("owner_loop"):
            return result(["publication loop does not own contract"])
        if not hasattr(reader,"pr") or not hasattr(reader,"publish"):
            return result(["publication authority producer unavailable"])
        errors=validate_contract_record(contract,target_repo_root,reader.pr(contract.get("approval_ref",{}).get("pr_number")))
        if errors:
            return result(errors)
        upstream=validate_contracts(tool_root,target_repo_root,{**binding,"input_contracts":contract["upstream_contracts"]},reader,purpose="new_tasks")
        if upstream["blockers"]:
            return upstream
        publication=(reader.publish(binding,contract,before_write=before_write) if before_write is not None else reader.publish(binding,contract))
        reference={"contract_id":contract["contract_id"],"revision":contract["revision"],"contract_digest":contract_digest(contract),"publication_ref":publication,"consumed_clauses":[c for item in contract["content_refs"] for c in item["clauses"]]}
        checked=validate_contracts(tool_root,target_repo_root,{**binding,"input_contracts":[reference]},reader,purpose="new_tasks")
        return {**checked,"publication_ref":publication,"input_contract":reference}
    except (ValueError,OSError,KeyError,TypeError) as exc:
        return result([str(exc)])
