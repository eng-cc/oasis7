#!/usr/bin/env python3
"""RED acceptance tests for trusted closed-duplicate candidate retirement.

The test harness uses an isolated Git repository and a fail-closed ``gh`` stub;
it never reads or writes live GitHub state.  The future helper's public CLI is
the one specified by source-of-truth.md:

    --mapping-root <registered-worktree> --task-uid <UID>
    --disposition-comment-id <server-comment-id> --preflight|--apply

Test-only seam contract (not a CLI/config/file input):
``retire_candidate(mapping_root, task_uid, disposition_comment_id, *, mode,
live_proof_provider)`` calls ``read_live_proof(mapping_root, task_uid)`` and
validates raw Issues, complete Project/comment/permission collections, the
server-ID comment readback, and complete Git/PR artifact-discovery evidence.
Incomplete or ambiguous values raise ``RetirementError`` without mutation.
The result statuses asserted here are ``preflight_ok``, ``retired``, and
``already_retired``. The production CLI must use the same validator and
transaction with a real fresh GitHub/Git provider; this injected provider is
for deterministic in-process tests only.
"""
from __future__ import annotations

import json
import hashlib
import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import copy
import unittest
import shutil
from unittest.mock import patch
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PM = ROOT / "scripts/pm"
HELPER = PM / "retire-closed-duplicate-candidate.py"
CANDIDATE_UID = "task_" + "a" * 32
FOREIGN_UID = "task_" + "b" * 32
CANDIDATE_ISSUE = 900001
REPLACEMENT_ISSUE = 900002
FOREIGN_BRANCH = "codex/foreign-owner-fixture"
FOREIGN_PR = 900003
DISPOSITION_MARKER = "<!-- oasis7.duplicate-candidate-disposition/v1 -->"


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def sha256(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def load_helper():
    if not HELPER.is_file():
        raise AssertionError("missing production behavior: trusted closed-duplicate retirement helper")
    spec = importlib.util.spec_from_file_location("retire_closed_duplicate_candidate_test", HELPER)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load trusted retirement helper at {HELPER}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(*args: str, cwd: Path | None = None) -> str:
    result = subprocess.run(
        ["git", *(str(arg) for arg in args)],
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise AssertionError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def candidate_record(foreign_worktree: Path) -> dict[str, object]:
    # This is the poisoned identity: the candidate row aliases a different,
    # nonterminal task's registered worktree and branch.
    return {
        "task_uid": CANDIDATE_UID,
        "repository": "eng-cc/oasis7",
        "issue_number": CANDIDATE_ISSUE,
        "issue_url": f"https://github.com/eng-cc/oasis7/issues/{CANDIDATE_ISSUE}",
        "project_item_id": "PVTI_candidate_fixture",
        "owner_role": "repository_health_engineer",
        "status": "candidate",
        "workflow_phase": "bootstrap",
        "canonical_worktree": str(foreign_worktree),
        "task_branch": FOREIGN_BRANCH,
        "pr_number": None,
        "pr_url": None,
    }


def foreign_record(foreign_worktree: Path) -> dict[str, object]:
    return {
        "task_uid": FOREIGN_UID,
        "repository": "eng-cc/oasis7",
        "issue_number": 900004,
        "issue_url": "https://github.com/eng-cc/oasis7/issues/900004",
        "project_item_id": "PVTI_foreign_fixture",
        "owner_role": "repository_health_engineer",
        "status": "committed",
        "workflow_phase": "execution",
        "canonical_worktree": str(foreign_worktree),
        "task_branch": FOREIGN_BRANCH,
        "pr_number": FOREIGN_PR,
        "pr_url": f"https://github.com/eng-cc/oasis7/pull/{FOREIGN_PR}",
    }


class RetirementFixture:
    """An isolated registered worktree pair and non-networking gh command."""

    def __init__(self, directory: Path):
        self.directory = directory
        self.repository = directory / "repository"
        self.mapping_root = directory / "candidate-worktree"
        self.foreign_worktree = directory / "foreign-worktree"
        self.bin = directory / "bin"
        self.mapping = self.mapping_root / ".pm/github-project-sync/tasks.json"
        self.gh_log = directory / "gh-calls.jsonl"
        self.git_log = directory / "git-calls.jsonl"
        self.gh_fixture = directory / "live-github-fixture.json"
        self.foreign_snapshot = self.foreign_worktree / ".pm/scratch" / FOREIGN_UID / "bootstrap-task-snapshot.json"
        self._create_git_worktrees()
        self._write_mapping()
        self._write_foreign_snapshot()
        self._write_gh_stub()

    def _create_git_worktrees(self) -> None:
        self.repository.mkdir(parents=True)
        git("init", "--quiet", "--initial-branch=main", str(self.repository))
        git("config", "user.email", "retirement-fixture@example.invalid", cwd=self.repository)
        git("config", "user.name", "Retirement Fixture", cwd=self.repository)
        (self.repository / "README.fixture").write_text("fixture\n", encoding="utf-8")
        git("add", "README.fixture", cwd=self.repository)
        git("commit", "--quiet", "-m", "retirement fixture", cwd=self.repository)
        git("remote", "add", "origin", "https://github.com/eng-cc/oasis7.git", cwd=self.repository)
        git("branch", FOREIGN_BRANCH, cwd=self.repository)
        git("worktree", "add", "--quiet", "--detach", str(self.mapping_root), "main", cwd=self.repository)
        git("worktree", "add", "--quiet", "--detach", str(self.foreign_worktree), FOREIGN_BRANCH, cwd=self.repository)

    def _write_mapping(self) -> None:
        self.mapping.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "version": 1,
            "project": {"repo": "eng-cc/oasis7", "owner": "eng-cc", "number": 1, "id": "PVT_fixture"},
            "tasks": {
                CANDIDATE_UID: candidate_record(self.foreign_worktree),
                FOREIGN_UID: foreign_record(self.foreign_worktree),
            },
        }
        self.mapping.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def _write_foreign_snapshot(self) -> None:
        snapshot = self.foreign_snapshot
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        payload: dict[str, object] = {
            "schema": "oasis7.bootstrap-task-snapshot/v1",
            "task": {"task_uid": FOREIGN_UID, "issue_number": 900004},
            "repository": "eng-cc/oasis7",
            "git": {"worktree": str(self.foreign_worktree), "branch": FOREIGN_BRANCH},
            "request": {"request_id": "foreign-fixture"},
            "producer": "test-fixture",
            "created_at": "2026-01-01T00:00:00Z",
        }
        payload["digest"] = sha256(canonical_bytes(payload))
        snapshot.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def write_candidate_artifact(self, kind: str) -> Path:
        """Create a real local artifact under the candidate UID for CLI scans."""
        task_scratch = self.foreign_worktree / ".pm/scratch" / CANDIDATE_UID
        if kind == "snapshot":
            path = task_scratch / "bootstrap-task-snapshot.json"
            payload: dict[str, object] = {
                "schema": "oasis7.bootstrap-task-snapshot/v1",
                "task": {"task_uid": CANDIDATE_UID, "issue_number": CANDIDATE_ISSUE},
                "repository": "eng-cc/oasis7",
                "git": {"worktree": str(self.foreign_worktree), "branch": FOREIGN_BRANCH},
                "request": {"request_id": "candidate-started-fixture"},
                "producer": "test-fixture",
                "created_at": "2026-09-26T00:00:00Z",
            }
            payload["digest"] = sha256(canonical_bytes(payload))
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            return path
        if kind == "execution":
            path = task_scratch / "slice-ledger.jsonl"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(
                json.dumps({"task_uid": CANDIDATE_UID, "role": "repository_health_engineer", "status": "completed", "head": "fixture-head"}) + "\n",
                encoding="utf-8",
            )
            return path
        if kind == "terminal":
            common = Path(git("rev-parse", "--git-common-dir", cwd=self.foreign_worktree))
            if not common.is_absolute():
                common = (self.foreign_worktree / common).resolve()
            path = common / "oasis7-workflow-receipts" / CANDIDATE_UID / "terminal-cleanup-receipt.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("{\"fixture\":\"candidate terminal receipt\"}\n", encoding="utf-8")
            return path
        raise AssertionError(f"unknown candidate artifact kind: {kind}")

    @staticmethod
    def issue_body(task_uid: str, status: str, phase: str, worktree: str = "", pr_number: str = "", pr_url: str = "") -> str:
        return "\n".join(
            [
                "<!-- oasis7-pm-task -->",
                f"task_uid: {task_uid}",
                "",
                "Task metadata:",
                f"- status: `{status}`",
                f"- workflow_phase: `{phase}`",
                f"- worktree_hint: `{worktree}`",
                f"- pr_number: `{pr_number}`",
                f"- pr_url: `{pr_url}`",
                "",
            ]
        )

    def live_proof(self) -> dict[str, object]:
        """Raw deterministic authority responses for the helper's injected test seam.

        The production CLI must never accept this structure from a file, flag,
        or environment variable. It must construct the same proof using fresh
        paginated GitHub and Git reads before calling the shared validator.
        """
        candidate = candidate_record(self.foreign_worktree)
        foreign = foreign_record(self.foreign_worktree)
        candidate_issue = {
            "repository": "eng-cc/oasis7",
            "number": CANDIDATE_ISSUE,
            "url": f"https://github.com/eng-cc/oasis7/issues/{CANDIDATE_ISSUE}",
            "state": "CLOSED",
            "state_reason": "DUPLICATE",
            "body": self.issue_body(CANDIDATE_UID, "candidate", "bootstrap"),
        }
        replacement_issue = {
            "repository": "eng-cc/oasis7",
            "number": REPLACEMENT_ISSUE,
            "url": f"https://github.com/eng-cc/oasis7/issues/{REPLACEMENT_ISSUE}",
            "state": "OPEN",
            "state_reason": None,
            "body": self.issue_body("task_" + "c" * 32, "committed", "execution", "/replacement", "", ""),
        }
        foreign_issue = {
            "repository": "eng-cc/oasis7",
            "number": int(foreign["issue_number"]),
            "url": str(foreign["issue_url"]),
            "state": "OPEN",
            "state_reason": None,
            "body": self.issue_body(FOREIGN_UID, "committed", "execution", str(self.foreign_worktree), str(FOREIGN_PR), str(foreign["pr_url"])),
        }

        def item(record: dict[str, object], fields: dict[str, str]) -> dict[str, object]:
            return {
                "id": str(record["project_item_id"]),
                "project_id": "PVT_fixture",
                "project_owner": "eng-cc",
                "project_number": 1,
                "repository": "eng-cc/oasis7",
                "issue_number": int(record["issue_number"]),
                "issue_url": str(record["issue_url"]),
                "task_uid": str(record["task_uid"]),
                "archived": False,
                "fields": fields,
            }

        candidate_item = item(
            candidate,
            {"Status": "Done", "PM Status": "candidate", "Workflow Phase": "bootstrap", "Canonical Worktree": ""},
        )
        replacement_item = item(
            {
                "project_item_id": "PVTI_replacement_fixture",
                "issue_number": REPLACEMENT_ISSUE,
                "issue_url": replacement_issue["url"],
                "task_uid": "task_" + "c" * 32,
            },
            {"Status": "In Progress", "PM Status": "committed", "Workflow Phase": "execution", "Canonical Worktree": "/replacement"},
        )
        foreign_item = item(
            foreign,
            {"Status": "In Progress", "PM Status": "committed", "Workflow Phase": "execution", "Canonical Worktree": str(self.foreign_worktree)},
        )
        candidate_binding = {
            "repository": "eng-cc/oasis7",
            "issue_number": CANDIDATE_ISSUE,
            "task_uid": CANDIDATE_UID,
            "project_item_id": candidate_item["id"],
        }
        replacement_binding = {
            "repository": "eng-cc/oasis7",
            "issue_number": REPLACEMENT_ISSUE,
            "task_uid": "task_" + "c" * 32,
            "project_item_id": replacement_item["id"],
        }
        disposition = {
            "candidate": candidate_binding,
            "reason": "duplicate",
            "no_source_work": True,
            "no_workflow_start": True,
            "no_worktree": True,
            "no_pr": True,
            "replacement": replacement_binding,
        }
        comment = {
            "id": 555001,
            "issue_number": CANDIDATE_ISSUE,
            "user": {"login": "fixture-admin"},
            "body": DISPOSITION_MARKER + "\n" + json.dumps(disposition, ensure_ascii=False, sort_keys=True, separators=(",", ":")),
        }
        foreign_pr = {
            "number": FOREIGN_PR,
            "state": "OPEN",
            "repository": "eng-cc/oasis7",
            "head_branch": FOREIGN_BRANCH,
            "head_repository": "eng-cc/oasis7",
            "issue_number": int(foreign["issue_number"]),
            "task_uid": FOREIGN_UID,
        }
        return {
            "repository": "eng-cc/oasis7",
            "project": {"owner": "eng-cc", "number": 1, "id": "PVT_fixture"},
            "issues": {"complete": True, "items": [candidate_issue, replacement_issue, foreign_issue]},
            "project_items": {"complete": True, "items": [candidate_item, replacement_item, foreign_item]},
            "candidate_comments": {
                "complete": True,
                "items": [
                    {"id": 555000, "issue_number": CANDIDATE_ISSUE, "user": {"login": "someone"}, "body": "ordinary unrelated comment"},
                    comment,
                ],
            },
            "comment_by_id": comment,
            "repository_permissions": {"complete": True, "items": [{"login": "fixture-admin", "permission": "admin"}]},
            "artifact_discovery": {
                "complete": True,
                "worktrees_complete": True,
                "branches_complete": True,
                "remote_branches_complete": True,
                "snapshots_complete": True,
                "execution_evidence_complete": True,
                "terminal_receipts_complete": True,
                "pull_requests_complete": True,
                "worktrees": [
                    {"path": str(self.mapping_root), "registered": True, "branch": None},
                    {"path": str(self.foreign_worktree), "registered": True, "branch": FOREIGN_BRANCH},
                ],
                "branches": [{"name": FOREIGN_BRANCH, "worktree": str(self.foreign_worktree), "task_uid": FOREIGN_UID}],
                "remotes": ["origin"],
                "remote_branches": [],
                "candidate_artifacts": {
                    "worktrees": [],
                    "branches": [],
                    "bootstrap_snapshots": [],
                    "execution_evidence": [],
                    "terminal_receipts": [],
                    "reciprocal_prs": [],
                },
                "foreign_owners": [
                    {
                        "task_uid": FOREIGN_UID,
                        "issue": foreign_issue,
                        "project_item": foreign_item,
                        "mapping_record": foreign,
                        "worktree": {"path": str(self.foreign_worktree), "registered": True, "branch": FOREIGN_BRANCH},
                        "snapshot": {"path": str(self.foreign_snapshot), "sha256": sha256(self.foreign_snapshot.read_bytes())},
                        "pull_requests": [foreign_pr],
                    }
                ],
                "pull_requests": [foreign_pr],
            },
        }

    def _write_gh_stub(self) -> None:
        self.bin.mkdir(parents=True)
        executable = self.bin / "gh"
        executable.write_text(
            "#!/bin/sh\n"
            "python3 -c 'import json,os,sys; "
            "open(os.environ[\"RETIREMENT_GH_LOG\"],\"a\",encoding=\"utf-8\").write(json.dumps(sys.argv[1:])+\"\\n\")'\n"
            "echo 'fixture: live GitHub authority unavailable' >&2\n"
            "exit 73\n",
            encoding="utf-8",
        )
        executable.chmod(0o755)

    def _write_paginated_live_gh_stub(self, pagination_mode: str = "complete", validate_project_owner_schema: bool = False) -> None:
        """Install fake `gh` and logging `git` executables for CLI integration.

        The fake GH transport answers ordinary Issue, ProjectV2, comment,
        permission, and PR reads from separate fixture resources. It does not
        return the normalized proof-provider object, so the CLI must delegate
        to its live collector and prove that all required pages were read.
        """
        proof = self.live_proof()
        self.gh_fixture.write_text(
            json.dumps(
                {
                    "issues": proof["issues"]["items"],
                    "project_items": proof["project_items"]["items"],
                    "comments": [
                        [proof["candidate_comments"]["items"][0]],
                        [proof["comment_by_id"]],
                    ],
                    "permission": proof["repository_permissions"]["items"][0],
                    # Slurp-style pagination is an array of page arrays; put
                    # the preserved foreign PR on page two so a first-page
                    # only read cannot falsely prove the candidate clean.
                    "pull_requests": [[], proof["artifact_discovery"]["pull_requests"]],
                    "pagination_mode": pagination_mode,
                    "validate_project_owner_schema": validate_project_owner_schema,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        self.bin.mkdir(parents=True, exist_ok=True)
        (self.bin / "gh").write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, pathlib, re, sys\n"
            "args=sys.argv[1:]\n"
            "with open(os.environ['RETIREMENT_GH_LOG'],'a',encoding='utf-8') as log: log.write(json.dumps(args)+'\\n')\n"
            "fixture=json.loads(pathlib.Path(os.environ['RETIREMENT_GH_FIXTURE']).read_text(encoding='utf-8'))\n"
            "mode=fixture['pagination_mode']\n"
            "def emit(value): print(json.dumps(value,sort_keys=True))\n"
            "def field_values(item):\n"
            "  return {'pageInfo':{'hasNextPage':False,'endCursor':None},'nodes':[{'name':v,'text':v,'field':{'name':k}} for k,v in item['fields'].items()]}\n"
            "def gql_item(item):\n"
            "  issue=next((x for x in fixture['issues'] if x['number']==item['issue_number']),{})\n"
            "  return {'id':item['id'],'isArchived':item.get('archived',False),'project':{'id':item['project_id'],'number':item['project_number'],'owner':{'login':item['project_owner']}},'content':{'__typename':'Issue','number':item['issue_number'],'url':item['issue_url'],'body':issue.get('body',''),'state':issue.get('state',''),'stateReason':issue.get('state_reason')},'fieldValues':field_values(item)}\n"
            "def graph(page):\n"
            "  items=fixture['project_items']\n"
            "  if page==0: selected=items[:2]; has_next=(mode=='project_incomplete' or len(items)>2); cursor=('retirement-cursor-1' if has_next and mode!='project_incomplete' else None)\n"
            "  else: selected=items[2:]; has_next=False; cursor=None\n"
            "  connection={'pageInfo':{'hasNextPage':has_next,'endCursor':cursor},'nodes':[gql_item(x) for x in selected]}\n"
            "  issue_cursor=(page>0 or 'retirement-issue-cursor-1' in ' '.join(args))\n"
            "  issue_nodes=fixture['issues'][2:] if issue_cursor else fixture['issues'][:2]\n"
            "  issue_has_next=(mode=='issues_incomplete' or (not issue_cursor and len(fixture['issues'])>2))\n"
            "  issue_end_cursor=('retirement-issue-cursor-1' if issue_has_next else None)\n"
            "  issue_connection={'nodes':issue_nodes,'pageInfo':{'hasNextPage':issue_has_next,'endCursor':issue_end_cursor}}\n"
            "  if mode=='issues_raw_list': issue_connection=fixture['issues']\n"
            "  project={'id':'PVT_fixture','number':1,'owner':{'login':'eng-cc'},'items':connection}\n"
            "  issue_number=900002 if '900002' in ' '.join(args) else 900001\n"
            "  selected_issue=next(x for x in fixture['issues'] if x['number']==issue_number)\n"
            "  selected_items=[gql_item(x) for x in fixture['project_items'] if x['issue_number']==issue_number]\n"
            "  selected_issue=dict(selected_issue,projectItems={'pageInfo':{'hasNextPage':False,'endCursor':None},'nodes':selected_items})\n"
            "  repository={'issue':selected_issue,'issues':issue_connection}\n"
            "  data={'organization':{'projectV2':project,'project':project},'repository':repository,'projectV2':project,'nodes':[gql_item(x) for x in selected]}\n"
            "  query=' '.join(args)\n"
            "  for alias in re.findall(r'\\b([A-Za-z_][A-Za-z_0-9]*):\\s*issue\\s*\\(',query):\n"
            "    key='replacement' if 'replacement' in alias.lower() else 'candidate'\n"
            "    number=900002 if key=='replacement' else 900001\n"
            "    issue=next(x for x in fixture['issues'] if x['number']==number)\n"
            "    item_nodes=[gql_item(x) for x in fixture['project_items'] if x['issue_number']==number]\n"
            "    data['repository'][alias]=dict(issue,projectItems={'pageInfo':{'hasNextPage':False,'endCursor':None},'nodes':item_nodes})\n"
            "  return {'data':data}\n"
            "if args[:2]==['api','graphql']:\n"
            "  joined=' '.join(args)\n"
            "  if fixture.get('validate_project_owner_schema') and 'projectV2(number:' in joined and re.search(r'owner\\s*\\{\\s*login\\s*\\}',joined) and '... on Organization' not in joined and '... on User' not in joined:\n"
            "    emit({'errors':[{'message':'Cannot query field login on ProjectV2Owner interface'}]}); raise SystemExit(1)\n"
            "  page=1 if 'retirement-cursor-1' in joined else 0\n"
            "  if '--paginate' in args and '--slurp' in args and mode=='complete': emit([graph(0),graph(1)])\n"
            "  elif '--paginate' in args and '--slurp' in args and mode=='issues_incomplete' and 'retirement-issue-cursor-1' in ' '.join(args): print('fixture: later Issue page unavailable',file=sys.stderr); sys.exit(73)\n"
            "  elif '--paginate' in args and '--slurp' in args: emit([graph(0)])\n"
            "  else: emit(graph(page))\n"
            "elif args[:1]==['api']:\n"
            "  route=next((a for a in args[1:] if not a.startswith('-')),'')\n"
            "  match=re.search(r'/issues/(\\d+)(?:/|$|[?])',route)\n"
            "  issue_number=int(match.group(1)) if match else None\n"
            "  if '/collaborators/' in route: emit(fixture['permission'])\n"
            "  elif '/issues/comments/' in route or re.search(r'/issues/\\d+/comments/\\d+$',route): emit(fixture['comments'][1][0])\n"
            "  elif issue_number and route.split('?')[0].endswith('/comments'):\n"
            "    if mode=='comments_incomplete': print('fixture: second comment page unavailable',file=sys.stderr); sys.exit(73)\n"
            "    emit(fixture['comments'] if '--paginate' in args and '--slurp' in args else fixture['comments'][0])\n"
            "  elif '/pulls' in route:\n"
            "    if mode=='pull_requests_incomplete': print('fixture: later PR page unavailable',file=sys.stderr); sys.exit(73)\n"
            "    emit(fixture['pull_requests'] if '--paginate' in args and '--slurp' in args else fixture['pull_requests'][0])\n"
            "  elif issue_number: emit(next(x for x in fixture['issues'] if x['number']==issue_number))\n"
            "  else: print('unexpected fixture API route: '+route,file=sys.stderr); sys.exit(73)\n"
            "elif args[:2]==['issue','view']:\n"
            "  number=int(args[2]); emit(next(x for x in fixture['issues'] if x['number']==number))\n"
            "elif args[:2]==['pr','list']:\n"
            "  if mode=='pull_requests_incomplete': print('fixture: later PR page unavailable',file=sys.stderr); sys.exit(73)\n"
            "  emit(fixture['pull_requests'] if '--paginate' in args and '--slurp' in args else fixture['pull_requests'][0])\n"
            "else:\n"
            "  print('unexpected live GitHub fixture command: '+json.dumps(args),file=sys.stderr); sys.exit(73)\n",
            encoding="utf-8",
        )
        (self.bin / "gh").chmod(0o755)
        real_git = shutil.which("git")
        if not real_git:
            raise AssertionError("git executable is required for isolated provider test")
        (self.bin / "git").write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, subprocess, sys\n"
            "with open(os.environ['RETIREMENT_GIT_LOG'],'a',encoding='utf-8') as log: log.write(json.dumps(sys.argv[1:])+'\\n')\n"
            "args=sys.argv[1:]; i=0\n"
            "while i<len(args) and args[i] in ('-C','-c','--git-dir','--work-tree'): i+=2\n"
            "command=args[i] if i<len(args) else ''; tail=args[i+1:]\n"
            "read_only={'rev-parse','check-ref-format','worktree','branch','for-each-ref','show-ref','config','symbolic-ref','status','cat-file','merge-base','ls-files','ls-tree','log','show','diff','remote','describe','rev-list','ls-remote'}\n"
            "mutating=command not in read_only or command in {'add','am','apply','checkout','clean','clone','commit','fetch','merge','mv','pull','push','rebase','reset','rm','stash','submodule','update-ref'}\n"
            "if command=='worktree' and (not tail or tail[0]!='list'): mutating=True\n"
            "if command=='branch' and any(x in ('-d','-D','--delete','-m','-M','--move','-c','-C','--copy','-f','--force') for x in tail): mutating=True\n"
            "if command=='branch' and tail and not any(x.startswith('-') for x in tail): mutating=True\n"
            "if command=='config' and any(x in ('--add','--unset','--unset-all','--replace-all','--remove-section','--rename-section') for x in tail): mutating=True\n"
            "if command=='config' and not any(x in ('--get','--get-all','--get-regexp','--get-urlmatch','--list','-l','--null','--show-origin','--show-scope') for x in tail) and len(tail)>1: mutating=True\n"
            "if command=='show-ref' and '--delete' in tail: mutating=True\n"
            "if command=='remote' and tail and tail[0] not in ('-v','--verbose','show','get-url'): mutating=True\n"
            "if command=='symbolic-ref' and len([x for x in tail if not x.startswith('-')])>1: mutating=True\n"
            "if mutating: print('fixture rejects mutating git command: '+json.dumps(args),file=sys.stderr); raise SystemExit(97)\n"
            "if command=='ls-remote':\n"
            "  refs=json.loads(os.environ.get('RETIREMENT_REMOTE_REFS_JSON','[]'))\n"
            "  for item in refs: print(item['oid']+'\\trefs/heads/'+item['branch'])\n"
            "  raise SystemExit(0)\n"
            "result=subprocess.run([os.environ['RETIREMENT_REAL_GIT'],*sys.argv[1:]])\n"
            "raise SystemExit(result.returncode)\n",
            encoding="utf-8",
        )
        (self.bin / "git").chmod(0o755)

    def _write_freeze_gh_stub(self) -> None:
        executable = self.bin / "gh"
        executable.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, pathlib, sys\n"
            "args=sys.argv[1:]\n"
            "with open(os.environ['RETIREMENT_GH_LOG'],'a',encoding='utf-8') as log: log.write(json.dumps(args)+'\\n')\n"
            "number=int(os.environ['FREEZE_ISSUE_NUMBER'])\n"
            "if args[:2]==['issue','view']:\n"
            "  fields=args[args.index('--json')+1].split(',')\n"
            "  if 'comments' in fields:\n"
            "    path=pathlib.Path(os.environ['FREEZE_COMMENT_BODY_FILE'])\n"
            "    body=path.read_text(encoding='utf-8') if path.exists() else ''\n"
            "    print(json.dumps({'comments':[{'body':body}] if body else []}))\n"
            "  else:\n"
            "    print(json.dumps({'number':number,'url':f'https://github.com/eng-cc/oasis7/issues/{number}','body':os.environ['FREEZE_ISSUE_BODY']}))\n"
            "elif args[:2]==['issue','comment']:\n"
            "  body_file=pathlib.Path(args[args.index('--body-file')+1])\n"
            "  pathlib.Path(os.environ['FREEZE_COMMENT_BODY_FILE']).write_text(body_file.read_text(encoding='utf-8'),encoding='utf-8')\n"
            "  print(f'https://github.com/eng-cc/oasis7/issues/{number}#issuecomment-123456')\n"
            "else:\n"
            "  print('unexpected freeze fixture command',file=sys.stderr); sys.exit(73)\n",
            encoding="utf-8",
        )
        executable.chmod(0o755)

    def run_draft_freeze(self) -> subprocess.CompletedProcess[str]:
        issue_body = self.issue_body(FOREIGN_UID, "committed", "execution", str(self.foreign_worktree), str(FOREIGN_PR), f"https://github.com/eng-cc/oasis7/pull/{FOREIGN_PR}")
        self._write_freeze_gh_stub()
        environment = self.environment()
        environment["FREEZE_ISSUE_NUMBER"] = str(foreign_record(self.foreign_worktree)["issue_number"])
        environment["FREEZE_ISSUE_BODY"] = issue_body
        environment["FREEZE_COMMENT_BODY_FILE"] = str(self.directory / "freeze-comment-body.txt")
        head = git("rev-parse", "HEAD", cwd=self.foreign_worktree)
        comparison_oid = git("rev-parse", "refs/heads/main", cwd=self.foreign_worktree)
        return subprocess.run(
            [
                sys.executable,
                str(PM / "record-draft-freeze-evidence.py"),
                "--worktree", str(self.foreign_worktree),
                "--branch", FOREIGN_BRANCH,
                "--head", head,
                "--comparison-ref", "refs/heads/main",
                "--comparison-oid", comparison_oid,
            ],
            cwd=ROOT,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
            timeout=10,
        )

    def environment(self) -> dict[str, str]:
        environment = dict(os.environ)
        environment["PATH"] = f"{self.bin}{os.pathsep}{environment.get('PATH', '')}"
        environment["RETIREMENT_GH_LOG"] = str(self.gh_log)
        environment["GH_PROMPT_DISABLED"] = "1"
        environment["GH_TOKEN"] = "fixture-token-never-sent"
        environment["RETIREMENT_REMOTE_REFS_JSON"] = "[]"
        if self.gh_fixture.is_file():
            environment["RETIREMENT_GH_FIXTURE"] = str(self.gh_fixture)
            environment["RETIREMENT_GIT_LOG"] = str(self.git_log)
            environment["RETIREMENT_REAL_GIT"] = shutil.which("git") or "git"
        return environment

    def run_helper(
        self,
        action: str,
        *,
        mapping_root: Path | None = None,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(HELPER),
                "--mapping-root",
                str(mapping_root or self.mapping_root),
                "--task-uid",
                CANDIDATE_UID,
                "--disposition-comment-id",
                "555001",
                f"--{action}",
            ],
            cwd=ROOT,
            env=self.environment(),
            text=True,
            capture_output=True,
            check=False,
            timeout=10,
        )


class FixtureLiveProofProvider:
    def __init__(self, proof: dict[str, object], on_read=None):
        self.proof = proof
        self.on_read = on_read
        self.read_count = 0

    def read_live_proof(self, mapping_root: Path, task_uid: str) -> dict[str, object]:
        self.read_count += 1
        if self.on_read is not None:
            self.on_read()
        return copy.deepcopy(self.proof)


def mutate_disposition(proof: dict[str, object], key: str, child_key: str | None, value: object) -> None:
    comment = proof["comment_by_id"]
    body = str(comment["body"])
    marker, separator, encoded = body.partition("\n")
    assert separator and marker == DISPOSITION_MARKER
    disposition = json.loads(encoded)
    if child_key is None:
        disposition[key] = value
    else:
        disposition[key][child_key] = value
    comment["body"] = marker + "\n" + json.dumps(disposition, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def make_noncompact_disposition(proof: dict[str, object]) -> None:
    comment = proof["comment_by_id"]
    _, _, encoded = str(comment["body"]).partition("\n")
    comment["body"] = DISPOSITION_MARKER + "\n" + json.dumps(json.loads(encoded), ensure_ascii=False, sort_keys=True, indent=2)


def make_replacement_terminal_without_receipts(proof: dict[str, object]) -> None:
    issue = proof["issues"]["items"][1]
    issue["state"] = "CLOSED"
    issue["state_reason"] = "COMPLETED"
    issue["body"] = str(issue["body"]).replace("- status: `committed`", "- status: `done`").replace(
        "- workflow_phase: `execution`", "- workflow_phase: `task_done`"
    )
    proof["project_items"]["items"][1]["fields"].update(
        {"Status": "Done", "PM Status": "done", "Workflow Phase": "task_done"}
    )


def run_test_seam(
    fixture: RetirementFixture,
    mode: str,
    proof: dict[str, object] | None = None,
    on_read=None,
    provider: FixtureLiveProofProvider | None = None,
):
    """Test-only seam contract; the live CLI must use its real proof provider."""
    helper = load_helper()
    function = getattr(helper, "retire_candidate", None)
    if not callable(function):
        raise AssertionError("missing shared retirement validator/transaction entrypoint: retire_candidate")
    if provider is None:
        provider = FixtureLiveProofProvider(proof if proof is not None else fixture.live_proof(), on_read=on_read)
    return function(
        mapping_root=fixture.mapping_root,
        task_uid=CANDIDATE_UID,
        disposition_comment_id=555001,
        mode=mode,
        live_proof_provider=provider,
    )


class RetirementHelperBoundaryTests(unittest.TestCase):
    def test_trusted_entrypoint_is_present_and_exposes_the_normative_cli(self):
        self.assertTrue(
            HELPER.is_file(),
            "missing production behavior: trusted closed-duplicate retirement helper",
        )
        result = subprocess.run(
            [sys.executable, str(HELPER), "--help"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=10,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        for flag in ("--mapping-root", "--task-uid", "--disposition-comment-id", "--preflight", "--apply"):
            self.assertIn(flag, result.stdout)
        for forbidden in ("--proof-file", "--fixture", "--trusted-tool-root", "--tool-root"):
            self.assertNotIn(forbidden, result.stdout, "live CLI must pin tools and refuse caller-supplied authority")

    def _assert_unavailable_authority_fails_without_mutation(self, action: str) -> None:
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-red-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            before = fixture.mapping.read_bytes()
            self.assertTrue(
                HELPER.is_file(),
                "missing production behavior: trusted closed-duplicate retirement helper",
            )
            result = fixture.run_helper(action)
            self.assertNotEqual(0, result.returncode, result.stdout)
            self.assertIn("fixture: live GitHub authority unavailable", result.stderr)
            self.assertEqual(before, fixture.mapping.read_bytes(), "failed authority reads must not mutate the mapping")
            self.assertFalse(list(fixture.mapping.parent.glob("tasks.json.tmp.*")))
            self.assertTrue(fixture.gh_log.is_file(), "helper did not attempt a live-authority read")
            calls = [json.loads(line) for line in fixture.gh_log.read_text(encoding="utf-8").splitlines()]
            self.assertTrue(calls)
            for call in calls:
                self.assertNotIn("POST", call)
                self.assertNotIn("PATCH", call)
                self.assertNotIn("DELETE", call)

    def test_preflight_is_read_only_and_blocks_when_live_authority_is_unavailable(self):
        self._assert_unavailable_authority_fails_without_mutation("preflight")

    def test_apply_repeats_live_checks_and_has_zero_partial_mutation_on_unavailable_authority(self):
        self._assert_unavailable_authority_fails_without_mutation("apply")

    def test_cli_rejects_symlinked_mapping_file_or_parent_without_external_writes(self):
        for symlink_kind in ("mapping_file", "mapping_parent"):
            for action in ("preflight", "apply"):
                with self.subTest(symlink_kind=symlink_kind, action=action), tempfile.TemporaryDirectory(
                    prefix="retire-duplicate-symlink-mapping-"
                ) as temporary:
                    fixture = RetirementFixture(Path(temporary))
                    fixture._write_paginated_live_gh_stub()
                    external = fixture.directory / "external"
                    external.mkdir()
                    if symlink_kind == "mapping_file":
                        external_mapping = external / "tasks.json"
                        external_mapping.write_bytes(fixture.mapping.read_bytes())
                        fixture.mapping.unlink()
                        fixture.mapping.symlink_to(external_mapping)
                    else:
                        external_mapping_dir = external / "github-project-sync"
                        fixture.mapping.parent.rename(external_mapping_dir)
                        fixture.mapping.parent.symlink_to(external_mapping_dir, target_is_directory=True)
                        external_mapping = external_mapping_dir / "tasks.json"

                    before = external_mapping.read_bytes()
                    external_lock = external_mapping.with_name(external_mapping.name + ".lock")
                    self.assertFalse(external_lock.exists())
                    result = fixture.run_helper(action)
                    failures = []
                    if result.returncode == 0:
                        failures.append(f"{action} unexpectedly accepted symlinked {symlink_kind}: {result.stdout.strip()}")
                    if not re.search(r"symlink|symbolic|mapping path|mapping file", result.stderr.lower()):
                        failures.append(f"{action} did not identify symlinked {symlink_kind}: {result.stderr.strip()}")
                    if before != external_mapping.read_bytes():
                        failures.append(f"{action} changed bytes outside the mapping root")
                    if external_lock.exists():
                        failures.append(f"{action} created external lock sidecar {external_lock}")
                    if list(external_mapping.parent.glob("tasks.json.tmp.*")):
                        failures.append(f"{action} left external mapping temporary files")
                    self.assertFalse(failures, "; ".join(failures))

    def test_cli_delegates_to_complete_paginated_github_and_git_fixture_reads(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-live-cli-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub()
            self.assertTrue(HELPER.is_file(), "missing production behavior: trusted closed-duplicate retirement helper")
            before = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            foreign_git_before = {
                "refs": git("show-ref", "--heads", cwd=fixture.repository),
                "worktrees": git("worktree", "list", "--porcelain", cwd=fixture.repository),
                "foreign_branch_oid": git("rev-parse", f"refs/heads/{FOREIGN_BRANCH}", cwd=fixture.repository),
                "foreign_readme": (fixture.foreign_worktree / "README.fixture").read_bytes(),
                "foreign_snapshot": fixture.foreign_snapshot.read_bytes(),
            }
            result = fixture.run_helper("apply")
            self.assertEqual(0, result.returncode, result.stderr)
            self.assertIn("retired", (result.stdout + result.stderr).lower())

            committed = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            self.assertNotIn(CANDIDATE_UID, committed["tasks"])
            self.assertEqual(before["tasks"][FOREIGN_UID], committed["tasks"][FOREIGN_UID])
            tombstone = committed["retired_duplicate_candidates"][0]
            evidence_text = json.dumps(tombstone.get("evidence"), sort_keys=True)
            self.assertIn("555001", evidence_text, "live comment-by-ID readback must be bound")
            self.assertIn(FOREIGN_UID, evidence_text, "foreign-owner live proof must be bound")
            self.assertIn(str(FOREIGN_PR), evidence_text, "foreign PR read must be bound")

            github_calls = fixture.gh_log.read_text(encoding="utf-8")
            parsed_gh_calls = [json.loads(line) for line in github_calls.splitlines()]
            graphql_calls = [call for call in parsed_gh_calls if call[:2] == ["api", "graphql"]]
            self.assertTrue(graphql_calls, "CLI did not query the canonical Project")
            graph_pagination_complete = any(
                "retirement-cursor-1" in " ".join(call)
                or ("--paginate" in call and "--slurp" in call)
                for call in graphql_calls
            )
            self.assertTrue(graph_pagination_complete, "Project query did not complete via cursor or gh paginator")
            self.assertIn("555001", github_calls, "CLI did not re-read disposition by server ID")
            self.assertIn("/collaborators/fixture-admin/permission", github_calls)
            self.assertNotRegex(github_calls, r'"(?:POST|PATCH|DELETE)"')
            pr_calls = [call for call in parsed_gh_calls if any("/pulls" in arg for arg in call)]
            self.assertTrue(pr_calls, "CLI did not discover pull requests through the REST collection")
            self.assertTrue(
                any("--paginate" in call and "--slurp" in call for call in pr_calls),
                "PR collection must be independently paginated and slurped",
            )

            git_calls = [json.loads(line) for line in fixture.git_log.read_text(encoding="utf-8").splitlines()]
            self.assertTrue(any("worktree" in call for call in git_calls), "CLI did not inspect registered worktrees")
            self.assertTrue(any("branch" in call for call in git_calls), "CLI did not inspect local branches")
            for call in git_calls:
                index = 0
                while index < len(call) and call[index] in ("-C", "-c", "--git-dir", "--work-tree"):
                    index += 2
                command = call[index] if index < len(call) else ""
                tail = call[index + 1:]
                self.assertIn(
                    command,
                    {"rev-parse", "check-ref-format", "worktree", "branch", "for-each-ref", "show-ref", "config", "symbolic-ref", "status", "cat-file", "merge-base", "ls-files", "ls-tree", "log", "show", "diff", "remote", "describe", "rev-list", "ls-remote"},
                    f"CLI invoked a non-read-only Git command: {call}",
                )
                self.assertNotIn(
                    command,
                    {"add", "am", "apply", "checkout", "clean", "clone", "commit", "fetch", "merge", "mv", "pull", "push", "rebase", "reset", "rm", "stash", "submodule", "update-ref"},
                    f"CLI attempted a mutating Git command: {call}",
                )
                if command == "worktree":
                    self.assertTrue(tail and tail[0] == "list", f"CLI attempted a worktree mutation: {call}")
                if command == "branch":
                    self.assertFalse(any(arg in ("-d", "-D", "--delete", "-m", "-M", "--move", "-f", "--force") for arg in tail), call)

            self.assertEqual(foreign_git_before["refs"], git("show-ref", "--heads", cwd=fixture.repository))
            self.assertEqual(foreign_git_before["worktrees"], git("worktree", "list", "--porcelain", cwd=fixture.repository))
            self.assertEqual(foreign_git_before["foreign_branch_oid"], git("rev-parse", f"refs/heads/{FOREIGN_BRANCH}", cwd=fixture.repository))
            self.assertEqual(foreign_git_before["foreign_readme"], (fixture.foreign_worktree / "README.fixture").read_bytes())
            self.assertEqual(foreign_git_before["foreign_snapshot"], fixture.foreign_snapshot.read_bytes())

    def test_cli_rejects_incomplete_pagination_with_zero_mapping_writes(self):
        for pagination_mode in ("project_incomplete", "comments_incomplete", "pull_requests_incomplete"):
            with self.subTest(pagination_mode=pagination_mode), tempfile.TemporaryDirectory(
                prefix="retire-duplicate-incomplete-live-cli-"
            ) as temporary:
                fixture = RetirementFixture(Path(temporary))
                fixture._write_paginated_live_gh_stub(pagination_mode)
                self.assertTrue(HELPER.is_file(), "missing production behavior: trusted closed-duplicate retirement helper")
                before = fixture.mapping.read_bytes()
                result = fixture.run_helper("apply")
                self.assertNotEqual(0, result.returncode, result.stdout)
                self.assertEqual(before, fixture.mapping.read_bytes())
                self.assertFalse(list(fixture.mapping.parent.glob("tasks.json.tmp.*")))
                calls = fixture.gh_log.read_text(encoding="utf-8")
                parsed_calls = [json.loads(line) for line in calls.splitlines()]
                if pagination_mode == "project_incomplete":
                    self.assertTrue(any(call[:2] == ["api", "graphql"] for call in parsed_calls))
                elif pagination_mode == "comments_incomplete":
                    comment_calls = [call for call in parsed_calls if any("/comments" in arg for arg in call)]
                    self.assertTrue(comment_calls)
                    self.assertTrue(any("--paginate" in call and "--slurp" in call for call in comment_calls))
                else:
                    pr_calls = [call for call in parsed_calls if any("/pulls" in arg for arg in call)]
                    self.assertTrue(pr_calls)
                    self.assertTrue(any("--paginate" in call and "--slurp" in call for call in pr_calls))

    def test_cli_rejects_malformed_or_truncated_repository_issue_connection(self):
        for pagination_mode in ("issues_raw_list", "issues_incomplete"):
            with self.subTest(pagination_mode=pagination_mode), tempfile.TemporaryDirectory(
                prefix="retire-duplicate-incomplete-issues-cli-"
            ) as temporary:
                fixture = RetirementFixture(Path(temporary))
                fixture._write_paginated_live_gh_stub(pagination_mode)
                before = fixture.mapping.read_bytes()

                result = fixture.run_helper("apply")

                self.assertNotEqual(0, result.returncode, result.stdout)
                self.assertEqual(before, fixture.mapping.read_bytes())
                calls = [json.loads(line) for line in fixture.gh_log.read_text(encoding="utf-8").splitlines()]
                issue_calls = [call for call in calls if call[:2] == ["api", "graphql"] and "issues(first:" in " ".join(call)]
                self.assertTrue(issue_calls, "CLI must independently enumerate repository Issues")
                self.assertTrue(any("--paginate" in call and "--slurp" in call for call in issue_calls))
                if pagination_mode == "issues_incomplete":
                    self.assertTrue(any("retirement-issue-cursor-1" in " ".join(call) for call in issue_calls))

    def test_cli_discovers_real_candidate_snapshot_execution_and_terminal_artifacts(self):
        expected_diagnostics = {
            "snapshot": r"snapshot|bootstrap-task-snapshot",
            "execution": r"execution|slice-ledger",
            "terminal": r"terminal|receipt",
        }
        for artifact_kind, diagnostic_pattern in expected_diagnostics.items():
            with self.subTest(artifact_kind=artifact_kind), tempfile.TemporaryDirectory(
                prefix="retire-duplicate-candidate-artifact-cli-"
            ) as temporary:
                fixture = RetirementFixture(Path(temporary))
                fixture._write_paginated_live_gh_stub()
                artifact_path = fixture.write_candidate_artifact(artifact_kind)
                artifact_before = artifact_path.read_bytes()
                mapping_before = fixture.mapping.read_bytes()
                foreign_refs_before = git("show-ref", "--heads", cwd=fixture.repository)
                foreign_worktrees_before = git("worktree", "list", "--porcelain", cwd=fixture.repository)
                self.assertTrue(HELPER.is_file(), "missing production behavior: trusted closed-duplicate retirement helper")

                result = fixture.run_helper("apply")
                self.assertNotEqual(0, result.returncode, result.stdout)
                self.assertRegex((result.stdout + result.stderr).lower(), diagnostic_pattern)
                self.assertEqual(mapping_before, fixture.mapping.read_bytes())
                self.assertEqual(artifact_before, artifact_path.read_bytes())
                self.assertEqual(foreign_refs_before, git("show-ref", "--heads", cwd=fixture.repository))
                self.assertEqual(foreign_worktrees_before, git("worktree", "list", "--porcelain", cwd=fixture.repository))
                self.assertFalse(list(fixture.mapping.parent.glob("tasks.json.tmp.*")))

                logged_git = [json.loads(line) for line in fixture.git_log.read_text(encoding="utf-8").splitlines()]
                self.assertTrue(any("worktree" in call for call in logged_git), "CLI did not perform live Git discovery")

class RetirementHelperBehaviorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.assertTrue(
            HELPER.is_file(),
            "missing production behavior: trusted closed-duplicate retirement helper",
        )

    def _assert_rejected_without_partial_mapping_write(self, fixture: RetirementFixture, proof: dict[str, object]) -> None:
        helper = load_helper()
        before = fixture.mapping.read_bytes()
        error = None
        try:
            run_test_seam(fixture, "apply", proof)
        except helper.RetirementError as exc:
            error = exc
        self.assertEqual(before, fixture.mapping.read_bytes(), "failed proof must not partially mutate mapping")
        self.assertIsNotNone(error, "unsafe proof must fail closed")

    def test_preflight_is_complete_but_byte_for_byte_read_only(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-preflight-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            before = fixture.mapping.read_bytes()
            result = run_test_seam(fixture, "preflight")
            self.assertEqual(before, fixture.mapping.read_bytes())
            self.assertFalse(list(fixture.mapping.parent.glob("tasks.json.tmp.*")))
            self.assertIn(result.get("status"), {"preflight_ok", "ready"})

    def test_apply_appends_bound_tombstone_atomically_preserves_foreign_owner_and_retry_is_idempotent(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-apply-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            before = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            snapshot_before = fixture.foreign_snapshot.read_bytes()
            result = run_test_seam(fixture, "apply")

            committed = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            self.assertEqual("retired", result.get("status"))
            self.assertNotIn(CANDIDATE_UID, committed["tasks"])
            self.assertEqual(before["tasks"][FOREIGN_UID], committed["tasks"][FOREIGN_UID])
            self.assertEqual(snapshot_before, fixture.foreign_snapshot.read_bytes())
            self.assertEqual(1, len(committed.get("retired_duplicate_candidates", [])))
            tombstone = committed["retired_duplicate_candidates"][0]
            self.assertEqual("oasis7.duplicate-candidate-retirement/v1", tombstone.get("schema"))
            self.assertEqual(CANDIDATE_UID, tombstone.get("task_uid"))
            self.assertEqual(before["tasks"][CANDIDATE_UID], tombstone.get("old_record"))
            self.assertEqual(sha256(canonical_bytes(tombstone["old_record"])), tombstone.get("old_record_sha256"))
            self.assertEqual("duplicate", tombstone.get("reason"))
            self.assertTrue(tombstone.get("transaction_id"))
            self.assertTrue(tombstone.get("issuer"))
            self.assertTrue(tombstone.get("time"))
            self.assertTrue(tombstone.get("evidence"))
            unsigned = dict(tombstone)
            digest = unsigned.pop("digest", None)
            self.assertEqual(sha256(canonical_bytes(unsigned)), digest)

            # The existing foreign branch, immutable snapshot, and PR are all
            # evidence to preserve, never candidate cleanup targets.
            evidence_text = json.dumps(tombstone["evidence"], sort_keys=True)
            comment_body = fixture.live_proof()["comment_by_id"]["body"]
            for bound_value in (
                CANDIDATE_UID,
                str(CANDIDATE_ISSUE),
                "PVTI_candidate_fixture",
                str(REPLACEMENT_ISSUE),
                "PVTI_replacement_fixture",
                "555001",
                sha256(comment_body.encode("utf-8")),
                "fixture-admin",
                FOREIGN_UID,
                FOREIGN_BRANCH,
                str(FOREIGN_PR),
                "PVTI_foreign_fixture",
                sha256(snapshot_before),
            ):
                self.assertIn(bound_value, evidence_text)
            once = fixture.mapping.read_bytes()
            class NoNewProofOnRetry:
                def read_live_proof(self, mapping_root: Path, task_uid: str):
                    raise AssertionError("retry must validate the durable tombstone, not manufacture replacement evidence")

            retry = run_test_seam(fixture, "apply", provider=NoNewProofOnRetry())
            self.assertIn(retry.get("status"), {"retired", "already_retired"})
            self.assertEqual(once, fixture.mapping.read_bytes(), "retry must not duplicate or rewrite the tombstone")

    def test_apply_repeats_live_proof_after_preflight_instead_of_reusing_preflight_result(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-fresh-apply-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            provider = FixtureLiveProofProvider(fixture.live_proof())
            run_test_seam(fixture, "preflight", provider=provider)
            self.assertEqual(1, provider.read_count)
            provider.proof["repository_permissions"]["items"][0]["permission"] = "write"
            before = fixture.mapping.read_bytes()
            helper = load_helper()
            error = None
            try:
                run_test_seam(fixture, "apply", provider=provider)
            except helper.RetirementError as exc:
                error = exc
            self.assertEqual(2, provider.read_count, "apply must repeat live reads after preflight")
            self.assertEqual(before, fixture.mapping.read_bytes())
            self.assertIsNotNone(error, "apply must fail on permission drift since preflight")

    def test_incomplete_or_ambiguous_live_authority_and_invalid_disposition_fail_without_partial_write(self):
        mutations = [
            ("incomplete Issue pagination", lambda p: p["issues"].update(complete=False)),
            ("ambiguous candidate Issue", lambda p: p["issues"]["items"].append(copy.deepcopy(p["issues"]["items"][0]))),
            ("incomplete Project pagination", lambda p: p["project_items"].update(complete=False)),
            ("duplicate candidate Project item", lambda p: p["project_items"]["items"].append(copy.deepcopy(p["project_items"]["items"][0]))),
            ("duplicate replacement Project item", lambda p: p["project_items"]["items"].append(copy.deepcopy(p["project_items"]["items"][1]))),
            ("incomplete comment pagination", lambda p: p["candidate_comments"].update(complete=False)),
            ("ambiguous disposition comments", lambda p: p["candidate_comments"]["items"].append(copy.deepcopy(p["candidate_comments"]["items"][1]))),
            ("comment ID/body mismatch", lambda p: p["comment_by_id"].update(body="different server body")),
            ("comment marker absent", lambda p: p["comment_by_id"].update(body="not a disposition")),
            ("noncanonical disposition JSON", lambda p: p["comment_by_id"].update(body=DISPOSITION_MARKER + "\n{")),
            ("valid but noncompact disposition JSON", lambda p: make_noncompact_disposition(p)),
            ("non-admin author", lambda p: p["repository_permissions"]["items"][0].update(permission="write")),
            ("incomplete permission read", lambda p: p["repository_permissions"].update(complete=False)),
            ("ambiguous permissions", lambda p: p["repository_permissions"]["items"].append({"login": "fixture-admin", "permission": "admin"})),
            ("wrong duplicate reason", lambda p: p["issues"]["items"][0].update(state_reason="NOT_PLANNED")),
            ("replacement is itself duplicate", lambda p: p["issues"]["items"][1].update(state="CLOSED", state_reason="DUPLICATE")),
            ("replacement repository mismatch", lambda p: p["issues"]["items"][1].update(repository="other/repo")),
            ("candidate not closed", lambda p: p["issues"]["items"][0].update(state="OPEN")),
            ("candidate Issue body says not candidate", lambda p: p["issues"]["items"][0].update(body=p["issues"]["items"][0]["body"].replace("status: `candidate`", "status: `committed`"))),
            ("candidate has workflow start", lambda p: p["issues"]["items"][0].update(body=p["issues"]["items"][0]["body"].replace("workflow_phase: `bootstrap`", "workflow_phase: `execution`"))),
            ("candidate has worktree hint", lambda p: p["issues"]["items"][0].update(body=p["issues"]["items"][0]["body"].replace("worktree_hint: ``", "worktree_hint: `/some/path`"))),
            ("candidate Issue has PR number", lambda p: p["issues"]["items"][0].update(body=p["issues"]["items"][0]["body"].replace("pr_number: ``", "pr_number: `101`"))),
            ("candidate Project status drift", lambda p: p["project_items"]["items"][0]["fields"].update({"PM Status": "committed"})),
            ("candidate Project Status is not Done", lambda p: p["project_items"]["items"][0]["fields"].update({"Status": "In Progress"})),
            ("candidate Project workflow phase drift", lambda p: p["project_items"]["items"][0]["fields"].update({"Workflow Phase": "execution"})),
            ("candidate Project worktree is set", lambda p: p["project_items"]["items"][0]["fields"].update({"Canonical Worktree": "/some/path"})),
            ("candidate Project item is archived", lambda p: p["project_items"]["items"][0].update(archived=True)),
            ("candidate Project item Issue mismatch", lambda p: p["project_items"]["items"][0].update(issue_number=CANDIDATE_ISSUE + 9)),
            ("candidate Project item UID mismatch", lambda p: p["project_items"]["items"][0].update(task_uid="task_" + "f" * 32)),
            ("candidate Project identity mismatch", lambda p: p["project_items"]["items"][0].update(project_id="PVT_other")),
            ("replacement Project item is archived", lambda p: p["project_items"]["items"][1].update(archived=True)),
            ("replacement Project item Issue mismatch", lambda p: p["project_items"]["items"][1].update(issue_number=REPLACEMENT_ISSUE + 9)),
            ("replacement Project item UID mismatch", lambda p: p["project_items"]["items"][1].update(task_uid="task_" + "f" * 32)),
            ("replacement Project repository mismatch", lambda p: p["project_items"]["items"][1].update(repository="other/repo")),
            ("replacement Project identity mismatch", lambda p: p["project_items"]["items"][1].update(project_id="PVT_other")),
            ("candidate binding repo mismatch", lambda p: mutate_disposition(p, "candidate", "repository", "other/repo")),
            ("candidate binding Issue mismatch", lambda p: mutate_disposition(p, "candidate", "issue_number", CANDIDATE_ISSUE + 1)),
            ("candidate binding UID mismatch", lambda p: mutate_disposition(p, "candidate", "task_uid", "task_" + "f" * 32)),
            ("candidate binding Project item mismatch", lambda p: mutate_disposition(p, "candidate", "project_item_id", "PVTI_other")),
            ("disposition replacement UID mismatch", lambda p: mutate_disposition(p, "replacement", "task_uid", "task_" + "f" * 32)),
            ("disposition no-source-work attestation false", lambda p: mutate_disposition(p, "no_source_work", None, False)),
            ("disposition no-workflow-start attestation false", lambda p: mutate_disposition(p, "no_workflow_start", None, False)),
            ("disposition no-worktree attestation false", lambda p: mutate_disposition(p, "no_worktree", None, False)),
            ("disposition no-PR attestation false", lambda p: mutate_disposition(p, "no_pr", None, False)),
            ("replacement Project item missing", lambda p: p["project_items"]["items"].pop(1)),
            ("terminal replacement lacks normal receipts", lambda p: make_replacement_terminal_without_receipts(p)),
            ("incomplete artifact discovery", lambda p: p["artifact_discovery"].update(complete=False)),
            ("incomplete worktree scan", lambda p: p["artifact_discovery"].update(worktrees_complete=False)),
            ("incomplete branch scan", lambda p: p["artifact_discovery"].update(branches_complete=False)),
            ("incomplete remote branch scan", lambda p: p["artifact_discovery"].update(remote_branches_complete=False)),
            ("unconfigured or malformed remote branch evidence", lambda p: p["artifact_discovery"]["remote_branches"].append({"remote": "missing", "name": "candidate", "oid": "stale"})),
            ("incomplete snapshot scan", lambda p: p["artifact_discovery"].update(snapshots_complete=False)),
            ("incomplete execution evidence scan", lambda p: p["artifact_discovery"].update(execution_evidence_complete=False)),
            ("incomplete terminal receipt scan", lambda p: p["artifact_discovery"].update(terminal_receipts_complete=False)),
            ("incomplete PR scan", lambda p: p["artifact_discovery"].update(pull_requests_complete=False)),
            ("foreign worktree branch mismatch", lambda p: p["artifact_discovery"]["foreign_owners"][0]["worktree"].update(branch="unexpected-branch")),
            ("foreign snapshot digest mismatch", lambda p: p["artifact_discovery"]["foreign_owners"][0]["snapshot"].update(sha256="sha256:stale")),
        ]
        for name, mutate in mutations:
            with self.subTest(name=name), tempfile.TemporaryDirectory(prefix="retire-duplicate-negative-") as temporary:
                fixture = RetirementFixture(Path(temporary))
                proof = fixture.live_proof()
                mutate(proof)
                self._assert_rejected_without_partial_mapping_write(fixture, proof)

    def test_any_candidate_owned_work_or_unregistered_artifact_blocks_with_zero_writes(self):
        artifact_keys = (
            "worktrees",
            "branches",
            "bootstrap_snapshots",
            "execution_evidence",
            "terminal_receipts",
            "reciprocal_prs",
        )
        for key in artifact_keys:
            with self.subTest(artifact=key), tempfile.TemporaryDirectory(prefix="retire-duplicate-artifact-") as temporary:
                fixture = RetirementFixture(Path(temporary))
                proof = fixture.live_proof()
                proof["artifact_discovery"]["candidate_artifacts"][key].append({"artifact": "candidate-owned"})
                self._assert_rejected_without_partial_mapping_write(fixture, proof)

    def test_exact_mapping_row_compare_and_swap_refuses_concurrent_change_without_overwriting_it(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-cas-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            proof = fixture.live_proof()
            concurrent = {}

            def concurrent_writer() -> None:
                payload = json.loads(fixture.mapping.read_text(encoding="utf-8"))
                payload["tasks"][CANDIDATE_UID]["updated_at"] = "2026-09-26T00:00:00Z"
                payload["tasks"][CANDIDATE_UID]["operator_annotation"] = "concurrent writer survives"
                fixture.mapping.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
                concurrent["bytes"] = fixture.mapping.read_bytes()

            helper = load_helper()
            error = None
            try:
                run_test_seam(fixture, "apply", proof, on_read=concurrent_writer)
            except helper.RetirementError as exc:
                error = exc
            self.assertEqual(concurrent["bytes"], fixture.mapping.read_bytes())
            self.assertIsNotNone(error, "exact-row CAS must reject a concurrently changed candidate row")
            payload = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            self.assertIn(CANDIDATE_UID, payload["tasks"])
            self.assertEqual("concurrent writer survives", payload["tasks"][CANDIDATE_UID]["operator_annotation"])

    def test_commit_replace_failure_leaves_mapping_and_temporary_files_unchanged(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-commit-failure-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            before = fixture.mapping.read_bytes()
            helper = load_helper()
            error = None
            try:
                with patch("os.replace", side_effect=OSError("fixture commit-stage failure")):
                    run_test_seam(fixture, "apply")
            except (helper.RetirementError, OSError) as exc:
                error = exc
            self.assertIsNotNone(error, "commit-stage failure must not be reported as retirement success")
            self.assertEqual(before, fixture.mapping.read_bytes())
            self.assertFalse(list(fixture.mapping.parent.glob("tasks.json.tmp.*")))
            payload = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            self.assertIn(CANDIDATE_UID, payload["tasks"])
            self.assertNotIn("retired_duplicate_candidates", payload)

    def test_unregistered_mapping_root_and_unknown_mode_fail_closed(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-unregistered-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            unregistered = Path(temporary) / "not-a-git-worktree"
            unregistered.mkdir()
            mapping = unregistered / ".pm/github-project-sync/tasks.json"
            mapping.parent.mkdir(parents=True)
            mapping.write_bytes(fixture.mapping.read_bytes())
            before = mapping.read_bytes()
            fixture_before = fixture.mapping.read_bytes()
            helper = load_helper()
            provider = FixtureLiveProofProvider(fixture.live_proof())
            unregistered_error = None
            try:
                helper.retire_candidate(
                    mapping_root=unregistered,
                    task_uid=CANDIDATE_UID,
                    disposition_comment_id=555001,
                    mode="apply",
                    live_proof_provider=provider,
                )
            except helper.RetirementError as exc:
                unregistered_error = exc
            self.assertEqual(before, mapping.read_bytes())
            self.assertIsNotNone(unregistered_error, "unregistered mapping root must be rejected")
            invalid_mode_error = None
            try:
                helper.retire_candidate(
                    mapping_root=fixture.mapping_root,
                    task_uid=CANDIDATE_UID,
                    disposition_comment_id=555001,
                    mode="force",
                    live_proof_provider=provider,
                )
            except helper.RetirementError as exc:
                invalid_mode_error = exc
            self.assertEqual(fixture_before, fixture.mapping.read_bytes())
            self.assertIsNotNone(invalid_mode_error, "only preflight and apply modes are valid")

    def test_refresh_bootstrap_and_selected_audit_recognize_retirement_without_resurrection(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-readers-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            run_test_seam(fixture, "apply")
            retired_mapping = fixture.mapping.read_bytes()

            refresh = subprocess.run(
                [
                    sys.executable, str(PM / "github-project-sync.py"), str(fixture.mapping_root),
                    "--repo", "eng-cc/oasis7", "--project-owner", "eng-cc", "--project-number", "1",
                    "--mapping", str(fixture.mapping), "--task-uid", CANDIDATE_UID,
                    "--dry-run", "--skip-recover", "--json",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=10,
            )
            self.assertEqual(0, refresh.returncode, refresh.stderr)
            self.assertIn("retired", (refresh.stdout + refresh.stderr).lower())
            self.assertEqual(retired_mapping, fixture.mapping.read_bytes(), "refresh must not recreate a tombstoned UID")

            bootstrap = subprocess.run(
                [
                    sys.executable, str(PM / "bootstrap-task-snapshot.py"), "validate-or-create",
                    "--repo-root", str(fixture.mapping_root), "--tasks-json", str(fixture.mapping),
                    "--task-uid", CANDIDATE_UID, "--producer", "retirement-regression-test",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=10,
            )
            self.assertNotEqual(0, bootstrap.returncode, "retired candidate must never bootstrap again")
            self.assertRegex(bootstrap.stderr.lower(), r"retired|tombstone")
            self.assertFalse((fixture.mapping_root / ".pm/scratch" / CANDIDATE_UID / "bootstrap-task-snapshot.json").exists())

            audit = subprocess.run(
                [
                    sys.executable, str(PM / "github-project-workflow.py"), str(fixture.mapping_root),
                    "--mapping", str(fixture.mapping), "audit", "--task-uid", CANDIDATE_UID, "--json",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=10,
            )
            report = audit.stdout + audit.stderr
            self.assertIn("retired", report.lower())
            self.assertNotIn("closed_without_merge", report)
            self.assertNotIn("task_complete", report)
            self.assertEqual(retired_mapping, fixture.mapping.read_bytes(), "audit must validate, not project completion or resurrect")

            workflow_next = subprocess.run(
                [
                    sys.executable, str(PM / "workflow-next.py"),
                    "--repo-root", str(fixture.mapping_root), "--mapping", str(fixture.mapping),
                    "--task-uid", CANDIDATE_UID, "--json",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=10,
            )
            self.assertNotEqual(0, workflow_next.returncode)
            self.assertRegex((workflow_next.stdout + workflow_next.stderr).lower(), r"retired|tombstone")
            self.assertEqual(retired_mapping, fixture.mapping.read_bytes(), "workflow-next must honor UID reservation")

    def test_all_mapping_readers_reject_a_corrupt_retirement_tombstone(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-corrupt-ledger-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub()
            run_test_seam(fixture, "apply")
            payload = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            payload["retired_duplicate_candidates"][0]["digest"] = "sha256:corrupt"
            fixture.mapping.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            corrupt_bytes = fixture.mapping.read_bytes()

            commands = (
                ("workflow-next", [sys.executable, str(PM / "workflow-next.py"), "--repo-root", str(fixture.mapping_root), "--mapping", str(fixture.mapping), "--task-uid", CANDIDATE_UID, "--json"]),
                ("bootstrap", [sys.executable, str(PM / "bootstrap-task-snapshot.py"), "validate-or-create", "--repo-root", str(fixture.mapping_root), "--tasks-json", str(fixture.mapping), "--task-uid", CANDIDATE_UID, "--producer", "retirement-regression-test"]),
                ("refresh", [sys.executable, str(PM / "github-project-sync.py"), str(fixture.mapping_root), "--repo", "eng-cc/oasis7", "--project-owner", "eng-cc", "--project-number", "1", "--mapping", str(fixture.mapping), "--task-uid", CANDIDATE_UID, "--dry-run", "--skip-recover", "--json"]),
                ("audit", [sys.executable, str(PM / "github-project-workflow.py"), str(fixture.mapping_root), "--mapping", str(fixture.mapping), "audit", "--task-uid", CANDIDATE_UID, "--json"]),
            )
            for name, command in commands:
                with self.subTest(reader=name):
                    result = subprocess.run(command, cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=10)
                    self.assertNotEqual(0, result.returncode, result.stdout)
                    self.assertRegex((result.stdout + result.stderr).lower(), r"tombstone|retirement|digest|corrupt")
                    self.assertEqual(corrupt_bytes, fixture.mapping.read_bytes(), "readers must not repair or rewrite corrupted authority")

    def test_draft_freeze_counts_exactly_one_unchanged_foreign_owner_after_retirement(self):
        with tempfile.TemporaryDirectory(prefix="retire-duplicate-freeze-") as temporary:
            fixture = RetirementFixture(Path(temporary))
            foreign_mapping = fixture.foreign_worktree / ".pm/github-project-sync/tasks.json"
            foreign_mapping.parent.mkdir(parents=True, exist_ok=True)
            foreign_mapping.write_bytes(fixture.mapping.read_bytes())
            blocked = fixture.run_draft_freeze()
            self.assertNotEqual(0, blocked.returncode)
            self.assertIn("found 2", blocked.stderr)

            run_test_seam(fixture, "apply")
            foreign_mapping.write_bytes(fixture.mapping.read_bytes())
            passed = fixture.run_draft_freeze()
            self.assertEqual(0, passed.returncode, passed.stderr)
            self.assertTrue((fixture.directory / "freeze-comment-body.txt").is_file())


if __name__ == "__main__":
    unittest.main(verbosity=2)
