#!/usr/bin/env python3
"""Task-bound draft PR publication entrypoint for the C1 ordered publisher."""
from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import json
import os
from contextlib import contextmanager
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import types
from typing import Any
from urllib.parse import urlencode

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from projection_publication_contract import ContractError, decode_marker
import pr_projection_journal
import pr_projection_publication as publication

LOCAL_COMMAND_TIMEOUT_SECONDS = 15.0
# Remote Git ref advertisement can take longer than the short local and GitHub
# admission reads; keep its bounded network budget separate from those limits.
REMOTE_GIT_READ_TIMEOUT_SECONDS = 30.0


class PublishInputError(RuntimeError):
    pass


@contextmanager
def _inherited_reservation(reservation_fd: int | None):
    """Keep the locked file handle open in a child that can outlive this process."""
    if reservation_fd is None:
        yield {}
        return
    if os.name == "nt":
        import msvcrt

        handle = msvcrt.get_osfhandle(reservation_fd)
        os.set_handle_inheritable(handle, True)
        startupinfo = subprocess.STARTUPINFO()
        startupinfo.lpAttributeList = {"handle_list": [handle]}
        try:
            yield {"close_fds": True, "startupinfo": startupinfo}
        finally:
            os.set_handle_inheritable(handle, False)
        return
    if os.name != "posix":
        raise PublishInputError("child publication reservation inheritance is unsupported")
    yield {"close_fds": True, "pass_fds": (reservation_fd,)}


def has_exact_task_pr_linkage(body: Any, task_uid: str, issue_number: int) -> bool:
    """Require one canonical whole-line Task marker and non-closing Refs line."""
    if not isinstance(body, str):
        return False
    lines = body.splitlines()
    task_lines = [line for line in lines if line.startswith("Task:")]
    refs_lines = [line for line in lines if line.startswith("Refs")]
    return (
        task_lines == [f"Task: {task_uid}"]
        and refs_lines == [f"Refs #{issue_number}"]
    )


def command_output(args: list[str], *, timeout: float = LOCAL_COMMAND_TIMEOUT_SECONDS,
                   reservation_fd: int | None = None) -> str:
    try:
        with _inherited_reservation(reservation_fd) as inherited:
            return subprocess.run(args, check=True, text=True, encoding="utf-8",
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  timeout=timeout, **inherited).stdout.strip()
    except subprocess.CalledProcessError as exc:
        detail = str(exc.stderr or "").strip()
        suffix = f": {detail[:500]}" if detail else f": {exc}"
        raise PublishInputError(f"command failed: {args[0]}{suffix}") from exc
    except (OSError, subprocess.SubprocessError) as exc:
        raise PublishInputError(f"command failed: {args[0]}: {exc}") from exc


def git(root: Path, *args: str) -> str:
    return command_output(["git", "-C", str(root), *args])


def load_projection_verifier(root: Path, authority_oid: str) -> Any:
    try:
        source = subprocess.check_output(
            ["git", "-C", str(root), "show",
             f"{authority_oid}:scripts/pm/workflow-impact-projection.py"],
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError as exc:
        raise PublishInputError("frozen planner authority lacks the trusted projection verifier") from exc
    module = types.ModuleType("c1_frozen_impact_projection_verifier")
    module.__file__ = f"{authority_oid}:scripts/pm/workflow-impact-projection.py"
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def repo_common_dir(root: Path) -> Path:
    value = Path(git(root, "rev-parse", "--git-common-dir"))
    return value.resolve() if value.is_absolute() else (root / value).resolve()


def mapping_identity(root: Path, uid: str, repo: str, issue_number: int,
                     branch: str, target_ref: str) -> dict[str, Any]:
    path = root / ".pm/github-project-sync/tasks.json"
    try:
        mapping = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PublishInputError(f"canonical task mapping is unreadable: {exc}") from exc
    tasks = mapping.get("tasks") if isinstance(mapping, dict) else None
    record = tasks.get(uid) if isinstance(tasks, dict) else None
    if not isinstance(record, dict):
        raise PublishInputError("canonical task mapping has no exact UID")
    expected = {
        "task_uid": uid, "repository": repo, "issue_number": issue_number,
        "task_branch": branch, "default_branch": target_ref,
        "canonical_worktree": str(root),
    }
    for key, value in expected.items():
        actual = record.get(key)
        if key == "canonical_worktree":
            matches = bool(actual) and Path(str(actual)).expanduser().resolve() == root
        else:
            matches = actual == value
        if not matches:
            raise PublishInputError(f"canonical task mapping {key} identity mismatch")
    return record


def _load_project_sync_module() -> Any:
    path = HERE / "github-project-sync.py"
    spec = importlib.util.spec_from_file_location("publication_project_sync", path)
    if spec is None or spec.loader is None:
        raise PublishInputError("canonical Project sync helper is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _mapping_project_identity(root: Path) -> dict[str, Any]:
    path = root / ".pm/github-project-sync/tasks.json"
    try:
        mapping = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PublishInputError(f"canonical task mapping is unreadable: {exc}") from exc
    project = mapping.get("project") if isinstance(mapping, dict) else None
    if not isinstance(project, dict):
        raise PublishInputError("canonical task mapping has no Project identity")
    return project


def task_publication(root: Path, args: argparse.Namespace) -> tuple[dict[str, Any], dict[str, Any]]:
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", args.repo):
        raise PublishInputError("repository identity is invalid")
    if not re.fullmatch(r"task_[0-9a-f]{32}", args.task_uid):
        raise PublishInputError("Task UID is invalid")
    if not re.fullmatch(r"[0-9a-f]{40,64}", args.source_head):
        raise PublishInputError("frozen source head is invalid")
    if not re.fullmatch(r"[0-9a-f]{40,64}", args.target_oid):
        raise PublishInputError("frozen target authority is invalid")
    if args.issue_number < 1:
        raise PublishInputError("Task issue number is invalid")
    command_output(["git", "-C", str(root), "check-ref-format", f"refs/heads/{args.source_ref}"])
    if args.remote.startswith("-") or not args.remote:
        raise PublishInputError("remote name is invalid")
    record = mapping_identity(root, args.task_uid, args.repo, args.issue_number,
                              args.source_ref, args.target_ref)
    if git(root, "rev-parse", "--verify", "HEAD^{commit}") != args.source_head:
        raise PublishInputError("current HEAD differs from frozen source head")
    if git(root, "rev-parse", "--verify", f"refs/heads/{args.source_ref}^{{commit}}") != args.source_head:
        raise PublishInputError("source branch differs from frozen source head")
    if git(root, "status", "--porcelain=v1"):
        raise PublishInputError("source worktree must be clean before publication")
    scope_oid = git(root, "merge-base", args.target_oid, args.source_head)
    verifier = load_projection_verifier(root, args.target_oid)
    projection = verifier.load_verified_projection(
        args.projection,
        expected={
            "task_uid": args.task_uid, "source_head_oid": args.source_head,
            "scope_base_oid": scope_oid,
        },
        repo_root=root,
    )
    try:
        authority_config = subprocess.check_output([
            "git", "-C", str(root), "show",
            f"{args.target_oid}:scripts/ci-required-scope.v2.json",
        ], stderr=subprocess.PIPE)
    except subprocess.CalledProcessError as exc:
        raise PublishInputError("frozen planner authority lacks its required-scope config") from exc
    config_digest = "sha256:" + hashlib.sha256(authority_config).hexdigest()
    if projection["planner_config_sha256"] != config_digest:
        raise PublishInputError("projection planner config differs from frozen target authority")
    try:
        repository_info = json.loads(command_output(["gh", "api", f"repos/{args.repo}"], timeout=10))
    except (json.JSONDecodeError, PublishInputError) as exc:
        raise PublishInputError(f"repository identity readback failed: {exc}") from exc
    repository_id = repository_info.get("id") if isinstance(repository_info, dict) else None
    if type(repository_id) is not int or repository_id < 1:
        raise PublishInputError("GitHub repository ID is unavailable")
    loop_binding = record.get("loop_binding")
    epoch = loop_binding.get("bootstrap_epoch") if isinstance(loop_binding, dict) else record.get("bootstrap_epoch")
    if epoch is not None and (type(epoch) is not int or epoch < 1):
        raise PublishInputError("canonical Task bootstrap epoch is invalid")
    value = publication.build_task_publication(
        repository=args.repo, repository_id=repository_id, task_uid=args.task_uid,
        bootstrap_epoch=epoch, source_repository_id=repository_id,
        source_ref=args.source_ref, target_ref=args.target_ref,
        source_head_oid=args.source_head, source_scope_oid=scope_oid,
        planner_authority_oid=args.target_oid,
        planner_config_sha256=projection["planner_config_sha256"],
        policy_digest=projection["planner_digest"],
        projection_digest=projection["projection_digest"],
    )
    candidate_projection = {
        "task_uid": projection["task_uid"],
        "source_head_oid": projection["source_head_oid"],
        "scope_base_oid": projection["scope_base_oid"],
        "planner_config_sha256": projection["planner_config_sha256"],
        "projection_digest": projection["projection_digest"],
        "consumed_contracts": projection["consumed_contracts"],
    }
    return value, candidate_projection


class GitHubPublicationAdapter:
    def __init__(self, root: Path, args: argparse.Namespace, candidate: dict[str, Any]):
        self.root = root
        self.args = args
        self.publication = candidate
        self.issue_number = args.issue_number
        self.task_helper = Path(args.task_helper).resolve()
        self.pr_number: int | None = None
        self.authenticated_login: str | None = None
        self.reservation_fd: int | None = None
        self.record_pr_recovery_required = False

    def gh(self, *args: str, timeout: float = 5.0,
           input_json: dict[str, Any] | None = None) -> str:
        input_text = json.dumps(input_json, ensure_ascii=False) if input_json is not None else None
        try:
            with _inherited_reservation(self.reservation_fd) as inherited:
                result = subprocess.run(["gh", *args], input=input_text, check=True, text=True,
                                        encoding="utf-8", stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, timeout=timeout, **inherited)
            return result.stdout.strip()
        except subprocess.CalledProcessError as exc:
            detail = str(exc.stderr or "").strip()
            raise RuntimeError(f"GitHub request failed: {detail[:500] if detail else exc}") from exc
        except (OSError, subprocess.SubprocessError) as exc:
            raise RuntimeError(f"GitHub request failed: {exc}") from exc

    def resolve_publisher_login(self) -> str:
        """Resolve the current authenticated GitHub identity without probing write permission."""
        raw = self.gh("api", "user", timeout=5.0)
        try:
            user = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise RuntimeError("authenticated GitHub user response is malformed") from exc
        login = user.get("login") if isinstance(user, dict) else None
        if not isinstance(login, str) or not login.strip() or any(ord(char) < 33 for char in login):
            raise RuntimeError("authenticated GitHub login is unavailable")
        self.authenticated_login = login
        return login

    def _issue_comments(self) -> list[dict[str, Any]]:
        raw = self.gh("api", f"repos/{self.args.repo}/issues/{self.issue_number}/comments?per_page=100",
                      "--paginate", "--slurp", timeout=30.0)
        try:
            pages = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise RuntimeError("Task comments readback is malformed") from exc
        if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
            raise RuntimeError("Task comments readback is malformed")
        values = [item for page in pages for item in page]
        if any(not isinstance(item, dict) or not isinstance(item.get("body"), str) for item in values):
            raise RuntimeError("Task comment entry is malformed")
        return values

    def _assert_task_identity(self) -> None:
        raw = self.gh("api", f"repos/{self.args.repo}/issues/{self.issue_number}", timeout=5.0)
        issue = json.loads(raw)
        if (not isinstance(issue, dict) or issue.get("number") != self.issue_number
                or str(issue.get("state") or "").lower() != "open"):
            raise RuntimeError("canonical Task issue is missing or closed")
        body = issue.get("body")
        if not isinstance(body, str):
            raise RuntimeError("canonical Task issue body is malformed")
        uids = re.findall(r"^task_uid:\s*(task_[0-9a-f]{32})$", body, re.MULTILINE)
        if uids != [self.args.task_uid]:
            raise RuntimeError("canonical Task issue UID mismatch")

    def _write_issue_comment(self, body: str) -> None:
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            handle.write(body)
            path = Path(handle.name)
        try:
            url = self.gh("issue", "comment", str(self.issue_number), "-R", self.args.repo,
                          "--body-file", str(path), timeout=15.0)
        finally:
            path.unlink(missing_ok=True)
        match = re.search(r"issuecomment-(\d+)(?:$|[?#])", url)
        if not match:
            raise RuntimeError("Issue comment response has no comment identity")
        readback = json.loads(self.gh("api", f"repos/{self.args.repo}/issues/comments/{match.group(1)}"))
        user = readback.get("user") if isinstance(readback, dict) else None
        author = user.get("login") if isinstance(user, dict) else None
        if (readback.get("body") != body
                or not isinstance(self.authenticated_login, str)
                or author != self.authenticated_login):
            raise RuntimeError("Issue comment exact author/content readback failed")
        if (publication._TASK_PUBLICATION_MARKER in body
                and not publication.comment_timestamps_are_unchanged(readback)):
            raise RuntimeError("C1 Issue comment timestamps are missing, malformed, or indicate an edit")

    def find_task_publications(self, publication_id: str) -> dict[str, Any]:
        self._assert_task_identity()
        target = self.publication
        matches = []
        authors = []
        bodies = []
        for comment in self._issue_comments():
            body = comment["body"]
            if "<!-- oasis7-ci-publication/v1 -->" not in body:
                continue
            value = publication.parse_publication_comment(body)
            if (value["task_uid"] == target["task_uid"]
                    and value["source_head_oid"] == target["source_head_oid"]
                    and value["source_scope_oid"] == target["source_scope_oid"]):
                if (value["publication_id"] == publication_id
                        and not publication.comment_timestamps_are_unchanged(comment)):
                    raise RuntimeError("existing C1 Issue comment timestamps are missing, malformed, or indicate an edit")
                matches.append(value)
                user = comment.get("user") if isinstance(comment.get("user"), dict) else {}
                authors.append({
                    "publication_id": value["publication_id"],
                    "author_login": user.get("login"),
                })
                bodies.append({"publication_id": value["publication_id"], "body": body})
        return {
            "complete": True, "publications": matches,
            "publication_authors": authors, "publication_bodies": bodies,
        }

    def publish_task_intent(self, value: dict[str, Any]) -> None:
        self._assert_task_identity()
        if not isinstance(self.authenticated_login, str) or not self.authenticated_login:
            raise RuntimeError("authenticated GitHub login was not resolved before POST")
        self._write_issue_comment(publication.publication_comment(value))

    def read_source_ref(self, source_ref: str) -> str | None:
        ref = f"refs/heads/{source_ref}"
        output = command_output(["git", "-C", str(self.root), "ls-remote",
                                 self.args.remote, ref], timeout=REMOTE_GIT_READ_TIMEOUT_SECONDS)
        if not output:
            return None
        rows = output.splitlines()
        if len(rows) != 1 or len(rows[0].split()) != 2 or rows[0].split()[1] != ref:
            raise RuntimeError("remote source ref readback is ambiguous")
        return rows[0].split()[0]

    def push_source_ref(self, source_ref: str, new_oid: str, lease_oid: str | None) -> None:
        if lease_oid is not None:
            result = subprocess.run(["git", "-C", str(self.root), "merge-base", "--is-ancestor",
                                     lease_oid, new_oid], check=False, capture_output=True, timeout=5)
            if result.returncode != 0:
                raise publication.PublicationError(
                    "PUBLICATION_WRITE_CONFLICT",
                    "frozen source head is not a fast-forward from the remote lease OID",
                )
            lease = f"--force-with-lease=refs/heads/{source_ref}:{lease_oid}"
        else:
            lease = f"--force-with-lease=refs/heads/{source_ref}:"
        command_output(["git", "-C", str(self.root), "push", self.args.remote, lease,
                        f"{new_oid}:refs/heads/{source_ref}"], timeout=60,
                       reservation_fd=self.reservation_fd)

    def find_task_prs(self, task_uid: str, source_ref: str, target_ref: str,
                      *, timeout_seconds: float = 5.0) -> dict[str, Any]:
        owner = self.args.repo.split("/", 1)[0]
        query = urlencode({
            "state": "all", "head": f"{owner}:{source_ref}",
            "base": target_ref, "per_page": 100,
        })
        raw = self.gh("api", f"repos/{self.args.repo}/pulls?{query}",
                      "--paginate", "--slurp", timeout=timeout_seconds)
        try:
            pages = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise RuntimeError("PR discovery response is malformed") from exc
        if not isinstance(pages, list):
            raise RuntimeError("PR discovery response is malformed")
        pull_requests = [item for page in pages for item in (page if isinstance(page, list) else [page])]
        matches = []
        for item in pull_requests:
            if not isinstance(item, dict):
                raise RuntimeError("PR discovery entry is malformed")
            head = item.get("head") if isinstance(item.get("head"), dict) else {}
            base = item.get("base") if isinstance(item.get("base"), dict) else {}
            head_repo = head.get("repo") if isinstance(head.get("repo"), dict) else {}
            base_repo = base.get("repo") if isinstance(base.get("repo"), dict) else {}
            if (head.get("ref") == source_ref and base.get("ref") == target_ref
                    and head_repo.get("full_name") == self.args.repo
                    and base_repo.get("full_name") == self.args.repo):
                matches.append({
                    "repository": self.args.repo, "source_ref": head["ref"],
                    "target_ref": base["ref"], "head_oid": head.get("sha"),
                    "body": item.get("body") or "", "state": str(item.get("state") or "").lower(),
                    "merged": bool(item.get("merged_at")), "draft": item.get("draft"),
                    "number": item.get("number"), "url": item.get("html_url"),
                })
        return {"complete": True, "pull_requests": matches}

    def create_draft_pr(self, repository: str, source_ref: str, target_ref: str, body: str) -> None:
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            handle.write(body)
            path = Path(handle.name)
        try:
            command = ["gh", "pr", "create", "-R", repository, "--base", target_ref,
                       "--head", source_ref, "--draft"]
            if self.args.title:
                command.extend(["--title", self.args.title])
            else:
                command.append("--fill")
            command.extend(["--body-file", str(path)])
            command_output(command, timeout=60, reservation_fd=self.reservation_fd)
        finally:
            path.unlink(missing_ok=True)

    def read_task_pr_binding(self, task_uid: str) -> dict[str, Any]:
        self._assert_task_identity()
        issue = json.loads(self.gh("api", f"repos/{self.args.repo}/issues/{self.issue_number}"))
        body = issue.get("body") if isinstance(issue, dict) else None
        if not isinstance(body, str):
            raise RuntimeError("Task issue body readback is malformed")
        uids = re.findall(r"^task_uid:\s*(task_[0-9a-f]{32})$", body, re.MULTILINE)
        if uids != [task_uid]:
            raise RuntimeError("Task issue UID readback mismatch")
        hold_active = re.findall(r"^- merge_hold_active:\s*`([^`]+)`$", body, re.MULTILINE)
        if (len(hold_active) > 1
                or (hold_active and hold_active[0].lower() not in {"true", "false"})):
            raise RuntimeError("Task merge hold readback is ambiguous")
        if hold_active == ["true"]:
            raise RuntimeError("active Task hold blocks PR publication")
        if getattr(self.args, "existing_ready_update", False):
            record = mapping_identity(self.root, task_uid, self.args.repo, self.issue_number,
                                      self.args.source_ref, self.args.target_ref)
            phase = (record.get("status"), record.get("workflow_phase"))
            if str(issue.get("state", "")).lower() != "open" or phase not in {
                    ("pr_watch", "pr_watch"), ("ready", "pre_pr_ready")}:
                raise RuntimeError("existing ready update requires OPEN admitted ready/pr_watch Task")
            for key, value in zip(("status", "workflow_phase"), phase):
                if re.findall(rf"^- {key}:\s*`([^`]+)`\s*$", body, re.MULTILINE) != [value]:
                    raise RuntimeError("live ready-update Task phase differs from mapping")
            if re.findall(r"^- worktree_hint:\s*`([^`]+)`\s*$", body, re.MULTILINE) != [str(self.root)]:
                raise RuntimeError("live ready-update Task worktree differs from mapping")
        url_lines = re.findall(r"^- pr_url:.*$", body, re.MULTILINE)
        if len(url_lines) > 1:
            raise RuntimeError("Task issue PR binding is ambiguous")
        if url_lines:
            match = re.fullmatch(r"- pr_url:\s*`([^`]+)`", url_lines[0])
            if match is None:
                raise RuntimeError("Task issue PR binding is malformed")
            number = publication.pr_number_from_url(match.group(1), self.args.repo)
        else:
            number = None
        if getattr(self.args, "existing_ready_update", False):
            if (number is None or record.get("pr_number") != number
                    or record.get("pr_url") != f"https://github.com/{self.args.repo}/pull/{number}"):
                raise RuntimeError("existing ready update requires exact recorded reciprocal PR")
            if re.findall(r"^- pr_number:\s*`([0-9]+)`\s*$", body, re.MULTILINE) != [str(number)]:
                raise RuntimeError("live ready-update Task PR number is missing or conflicting")
            live_pr = self.read_pr(self.args.repo, number)
            publication._check_pr(live_pr, self.publication, number, live_pr.get("head_oid"), False)
            if not has_exact_task_pr_linkage(
                live_pr.get("body"), task_uid, self.issue_number,
            ):
                raise RuntimeError("live ready-update PR lacks exact unique Task/Refs identity")
        if number is not None:
            self.pr_number = number
        binding = {"task_uid": task_uid, "pr_number": number}
        if getattr(self.args, "existing_ready_update", False):
            binding["existing_ready_update"] = True
        return binding

    def read_completed_task_publication_state(self, value: dict[str, Any], number: int,
                                              expected_draft: bool) -> dict[str, Any]:
        issue = json.loads(self.gh("api", f"repos/{self.args.repo}/issues/{self.issue_number}", timeout=5.0))
        body = issue.get("body") if isinstance(issue, dict) else None
        if (not isinstance(issue, dict) or issue.get("number") != self.issue_number
                or str(issue.get("state") or "").lower() != "open"
                or not isinstance(body, str)):
            raise RuntimeError("canonical Task issue is unavailable for completed replay")
        uids = re.findall(r"^task_uid:\s*(task_[0-9a-f]{32})$", body, re.MULTILINE)
        if uids != [value["task_uid"]]:
            raise RuntimeError("completed replay Task UID readback mismatch")
        hold_active = re.findall(r"^- merge_hold_active:\s*`([^`]+)`$", body, re.MULTILINE)
        if (len(hold_active) > 1
                or (hold_active and hold_active[0].lower() not in {"true", "false"})
                or hold_active == ["true"]):
            raise RuntimeError("completed replay Task hold readback is active or ambiguous")
        task_status = re.findall(r"^- status:\s*`([^`]+)`\s*$", body, re.MULTILINE)
        task_phase = re.findall(r"^- workflow_phase:\s*`([^`]+)`\s*$", body, re.MULTILINE)
        pr_urls = re.findall(r"^- pr_url:\s*`([^`]+)`\s*$", body, re.MULTILINE)
        pr_numbers = re.findall(r"^- pr_number:\s*`([0-9]+)`\s*$", body, re.MULTILINE)
        if len(task_status) != 1 or len(task_phase) != 1 or len(pr_urls) != 1 or len(pr_numbers) != 1:
            raise RuntimeError("completed replay Task lifecycle/PR binding is incomplete")
        task_pr_number = int(pr_numbers[0])
        task_pr_url = pr_urls[0]
        if task_pr_number != number or task_pr_url != f"https://github.com/{self.args.repo}/pull/{number}":
            raise RuntimeError("completed replay Task PR binding mismatches the candidate")
        record = mapping_identity(self.root, value["task_uid"], self.args.repo, self.issue_number,
                                  self.args.source_ref, self.args.target_ref)
        if (record.get("status") != task_status[0]
                or record.get("workflow_phase") != task_phase[0]
                or record.get("pr_number") != number
                or record.get("pr_url") != task_pr_url
                or record.get("merge_hold", {}).get("active") is not False):
            raise RuntimeError("completed replay local Task mapping differs from live Task")
        project = self.read_completed_project_state(record, task_pr_url)
        pr = self.read_pr(value["repository"], number)
        if not has_exact_task_pr_linkage(pr.get("body"), value["task_uid"], self.issue_number):
            raise RuntimeError("completed replay PR lacks exact Task/Refs linkage")
        user = issue.get("user") if isinstance(issue.get("user"), dict) else {}
        pr_binding = {
            "repository": value["repository"],
            "number": number,
            "url": f"https://github.com/{value['repository']}/pull/{number}",
            "state": pr.get("state"),
            "merged": pr.get("merged"),
            "draft": pr.get("draft"),
            "source_ref": pr.get("source_ref"),
            "target_ref": pr.get("target_ref"),
            "source_head_oid": pr.get("head_oid"),
            "task_uid": value["task_uid"],
            "issue_number": self.issue_number,
            "created_at": pr.get("created_at"),
            "updated_at": pr.get("updated_at"),
            "task_status": task_status[0],
            "task_phase": task_phase[0],
            "task_pr_number": task_pr_number,
            "task_pr_url": task_pr_url,
            "pr_author": pr.get("pr_author"),
            "pr_author_type": pr.get("pr_author_type"),
        }
        if pr_binding["draft"] is not expected_draft:
            raise RuntimeError("completed replay PR draft state differs from expected publication mode")
        comments = self._issue_comments()
        return {
            "comments_read": {
                "complete": True, "repository": value["repository"],
                "issue_number": self.issue_number, "comments": comments,
            },
            "live_task_author": {"login": user.get("login"), "type": user.get("type")},
            "pr_binding": pr_binding,
            "project": project,
            "bindings_read": self.find_task_publication_bindings(value["publication_id"]),
            "pr": pr,
            "expected_pr_body": pr.get("body"),
        }

    def read_completed_project_state(self, record: dict[str, Any], task_pr_url: str) -> dict[str, Any]:
        project = _mapping_project_identity(self.root)
        project_id = project.get("id")
        project_owner = project.get("owner")
        project_number = project.get("number")
        item_id = record.get("project_item_id")
        if (not isinstance(project_id, str) or not project_id
                or not isinstance(project_owner, str) or not project_owner
                or type(project_number) is not int or project_number < 1
                or not isinstance(item_id, str) or not item_id):
            raise publication.PublicationError(
                "TASK_IDENTITY_CONFLICT", "completed replay Project mapping identity is incomplete",
            )
        owner, name = self.args.repo.split("/", 1)
        membership_query = """
        query($owner: String!, $name: String!, $number: Int!, $after: String) {
          repository(owner: $owner, name: $name) {
            issue(number: $number) {
              id
              number
              url
              state
              projectItems(first: 100, after: $after) {
                nodes {
                  id
                  isArchived
                  project {
                    id
                    number
                    viewerCanUpdate
                    owner { ... on Organization { login } ... on User { login } }
                  }
                }
                pageInfo { hasNextPage endCursor }
              }
            }
          }
        }
        """
        memberships: list[dict[str, Any]] = []
        after: str | None = None
        seen_cursors: set[str] = set()
        issue_identity = None
        for _ in range(100):
            raw_membership = self.gh(
                "api", "graphql",
                "-f", "query=" + membership_query,
                "-F", "owner=" + owner,
                "-F", "name=" + name,
                "-F", "number=" + str(self.issue_number),
                "-F", "after=" + (after or ""),
                timeout=10.0,
            )
            try:
                payload = json.loads(raw_membership)
            except json.JSONDecodeError as exc:
                raise RuntimeError("completed replay Issue Project membership readback is malformed") from exc
            data = payload.get("data") if isinstance(payload, dict) else None
            repository = data.get("repository") if isinstance(data, dict) else None
            issue = repository.get("issue") if isinstance(repository, dict) else None
            if not isinstance(issue, dict):
                raise publication.PublicationError(
                    "TASK_IDENTITY_CONFLICT", "completed replay Task Issue membership is unavailable",
                )
            current_issue = {key: issue.get(key) for key in ("id", "number", "url", "state")}
            if issue_identity is None:
                issue_identity = current_issue
            elif current_issue != issue_identity:
                raise publication.PublicationError(
                    "NETWORK_UNCERTAIN",
                    "completed replay Task Issue changed during Project membership pagination",
                )
            connection = issue.get("projectItems")
            nodes = connection.get("nodes") if isinstance(connection, dict) else None
            page_info = connection.get("pageInfo") if isinstance(connection, dict) else None
            if (not isinstance(nodes, list) or not isinstance(page_info, dict)
                    or type(page_info.get("hasNextPage")) is not bool):
                raise publication.PublicationError(
                    "NETWORK_UNCERTAIN", "completed replay Project membership pagination is incomplete",
                )
            if any(not isinstance(item, dict) or not isinstance(item.get("id"), str)
                   or not item.get("id") or not isinstance(item.get("project"), dict)
                   for item in nodes):
                raise publication.PublicationError(
                    "TASK_IDENTITY_CONFLICT", "completed replay Project membership entry is malformed",
                )
            memberships.extend(nodes)
            if not page_info["hasNextPage"]:
                break
            cursor = page_info.get("endCursor")
            if not isinstance(cursor, str) or not cursor or cursor in seen_cursors:
                raise publication.PublicationError(
                    "NETWORK_UNCERTAIN", "completed replay Project membership cursor is missing or repeated",
                )
            seen_cursors.add(cursor)
            after = cursor
        else:
            raise publication.PublicationError(
                "NETWORK_UNCERTAIN", "completed replay Project membership pagination limit exhausted",
            )
        if (not isinstance(issue_identity, dict)
                or issue_identity.get("number") != self.issue_number
                or issue_identity.get("url") != f"https://github.com/{self.args.repo}/issues/{self.issue_number}"
                or str(issue_identity.get("state") or "").upper() != "OPEN"):
            raise publication.PublicationError(
                "TASK_IDENTITY_CONFLICT", "completed replay Task Issue identity/state is invalid",
            )
        membership_matches = [
            item for item in memberships
            if item.get("id") == item_id and item.get("project", {}).get("id") == project_id
        ]
        if len(membership_matches) != 1:
            raise publication.PublicationError(
                "TASK_IDENTITY_CONFLICT",
                "completed replay cached Project item is not the live Task Issue Project item",
            )
        membership_project = membership_matches[0]["project"]
        membership_owner = membership_project.get("owner")
        if (membership_matches[0].get("isArchived") is not False
                or membership_project.get("number") != project_number
                or not isinstance(membership_owner, dict)
                or membership_owner.get("login") != project_owner):
            raise publication.PublicationError(
                "TASK_IDENTITY_CONFLICT",
                "completed replay Project membership identity is incomplete",
            )
        query = """
        query($item: ID!, $after: String) {
          node(id: $item) {
            ... on ProjectV2Item {
              id
              isArchived
              project {
                id
                number
                viewerCanUpdate
                owner { ... on Organization { login } ... on User { login } }
              }
              fieldValues(first: 100, after: $after) {
                nodes {
                  ... on ProjectV2ItemFieldTextValue {
                    text
                    field { ... on ProjectV2FieldCommon { name } }
                  }
                  ... on ProjectV2ItemFieldSingleSelectValue {
                    name
                    field { ... on ProjectV2FieldCommon { name } }
                  }
                }
                pageInfo { hasNextPage endCursor }
              }
            }
          }
        }
        """
        live_values: dict[str, str] = {}
        after = None
        seen_cursors = set()
        item_identity = None
        for _ in range(100):
            raw = self.gh(
                "api", "graphql", "-f", "query=" + query,
                "-F", "item=" + item_id, "-F", "after=" + (after or ""),
                timeout=10.0,
            )
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError as exc:
                raise RuntimeError("completed replay Project readback is malformed") from exc
            data = payload.get("data") if isinstance(payload, dict) else None
            node = data.get("node") if isinstance(data, dict) else None
            if not isinstance(node, dict):
                raise publication.PublicationError(
                    "TASK_IDENTITY_CONFLICT", "completed replay Project item readback is unavailable",
                )
            current_identity = {
                "id": node.get("id"), "isArchived": node.get("isArchived"),
                "project": node.get("project"),
            }
            if item_identity is None:
                item_identity = current_identity
            elif current_identity != item_identity:
                raise publication.PublicationError(
                    "NETWORK_UNCERTAIN", "completed replay Project item changed during field pagination",
                )
            field_values = node.get("fieldValues")
            page_info = field_values.get("pageInfo") if isinstance(field_values, dict) else None
            nodes = field_values.get("nodes") if isinstance(field_values, dict) else None
            if (not isinstance(page_info, dict)
                    or type(page_info.get("hasNextPage")) is not bool
                    or not isinstance(nodes, list)):
                raise publication.PublicationError(
                    "NETWORK_UNCERTAIN", "completed replay Project field readback is incomplete",
                )
            for value in nodes:
                field = value.get("field") if isinstance(value, dict) else None
                name = field.get("name") if isinstance(field, dict) else None
                if not isinstance(name, str) or not name:
                    raise publication.PublicationError(
                        "TASK_IDENTITY_CONFLICT", "completed replay Project field value is malformed",
                    )
                if name in live_values:
                    raise publication.PublicationError(
                        "TASK_IDENTITY_CONFLICT", "completed replay Project field value is duplicated",
                    )
                raw = value.get("name")
                if raw is None:
                    raw = value.get("text")
                if raw is not None and not isinstance(raw, str):
                    raise publication.PublicationError(
                        "TASK_IDENTITY_CONFLICT", "completed replay Project field value is malformed",
                    )
                live_values[name] = raw or ""
            if not page_info["hasNextPage"]:
                break
            cursor = page_info.get("endCursor")
            if not isinstance(cursor, str) or not cursor or cursor in seen_cursors:
                raise publication.PublicationError(
                    "NETWORK_UNCERTAIN", "completed replay Project field cursor is missing or repeated",
                )
            seen_cursors.add(cursor)
            after = cursor
        else:
            raise publication.PublicationError(
                "NETWORK_UNCERTAIN", "completed replay Project field pagination limit exhausted",
            )
        item_project = item_identity.get("project") if isinstance(item_identity, dict) else None
        item_owner = item_project.get("owner") if isinstance(item_project, dict) else None
        if (not isinstance(item_identity, dict)
                or item_identity.get("id") != item_id
                or item_identity.get("isArchived") is not False
                or not isinstance(item_project, dict)
                or item_project.get("id") != project_id
                or item_project.get("number") != project_number
                or not isinstance(item_owner, dict)
                or item_owner.get("login") != project_owner):
            raise publication.PublicationError(
                "TASK_IDENTITY_CONFLICT",
                "completed replay Project item identity readback is incomplete",
            )
        sync = _load_project_sync_module()
        expected_record = dict(record)
        expected_record.update({
            "status": "committed",
            "workflow_phase": "verification",
            "pr_url": task_pr_url,
            "pr_number": publication.pr_number_from_url(task_pr_url, self.args.repo),
        })
        expected = sync.project_field_values(expected_record)
        required = {key: value for key, value in expected.items() if key in {
            "Task UID", "Status", "PM Status", "Workflow Phase", "PR", "Canonical Worktree",
            "Owner Role", "Module", "Priority", "Test Tier Required",
        }}
        missing = [key for key in required if key not in live_values]
        drift = [key for key, value in required.items() if live_values.get(key) != value]
        if missing or drift:
            raise publication.PublicationError(
                "TASK_IDENTITY_CONFLICT",
                "completed replay live Project fields differ from the completed Task vector",
            )
        return {
            "status": "passed",
            "project_id": project_id,
            "project_owner": project_owner,
            "project_number": project_number,
            "project_item_id": item_id,
            "field_count": len(live_values),
            "required_fields": sorted(required),
        }

    def require_record_pr_recovery_admission(self) -> None:
        """Require a unique live recovery marker before retrying record-pr."""
        if not self.task_helper.is_file():
            raise RuntimeError("canonical recovery helper is unavailable")
        marker = "<!-- oasis7-publication-recovery-admission/v1 -->"
        comments = self._issue_comments()
        if sum(marker in comment["body"] for comment in comments) != 1:
            raise RuntimeError("one unique current record-pr recovery admission is required")
        # record_pr() re-reads comments and reconstructs the full authority
        # before helper launch, so a changed/removed admission fails closed.
        self.record_pr_recovery_required = True

    def record_pr(self, task_uid: str, number: int, publication_id: str) -> None:
        url = f"https://github.com/{self.args.repo}/pull/{number}"
        binding = publication.build_publication_binding(self.publication, number, url)
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            json.dump(binding, handle, ensure_ascii=False, sort_keys=True)
            path = Path(handle.name)
        try:
            command = [
                sys.executable, str(self.task_helper), "record-pr", str(self.root),
                "--repo", self.args.repo,
                "--task-uid", task_uid, "--pr-url", url, "--role", "tpm",
                "--validation-command", "C1 ordered CI projection publication",
                "--existing-ready-update" if getattr(self.args, "existing_ready_update", False) else "--draft-candidate",
                "--publication-binding-json", str(path), "--json",
            ]
            if self.record_pr_recovery_required:
                # Preserve the parent's required-recovery mode across the
                # process boundary. The child still reconstructs authority
                # from its own fresh authenticated read; this flag only
                # prevents a revoked marker from selecting the ordinary path.
                command.append("--recovery-required")
            # The publisher derives recovery authority from live authenticated
            # comments and pure reads of the existing journals. It transports
            # no caller permission grant; record-pr recomputes it independently.
            recovery = None
            # An unavailable helper cannot execute recovery. Leave its normal
            # command failure intact before making discovery network requests.
            comments = self._issue_comments() if self.task_helper.is_file() else []
            marker = "<!-- oasis7-publication-recovery-admission/v1 -->"
            admissions = [c for c in comments if marker in c["body"]]
            if self.record_pr_recovery_required and len(admissions) != 1:
                raise RuntimeError("required record-pr recovery admission changed before helper launch")
            if admissions:
                if self.task_helper != (self.root / "scripts/pm/github-project-task.py").resolve():
                    raise RuntimeError("recovery requires the canonical reviewed task helper")
                spec = importlib.util.spec_from_file_location("publication_task_recovery_impl", self.task_helper)
                if spec is None or spec.loader is None:
                    raise RuntimeError("canonical task recovery helper unavailable")
                helper = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(helper)
                selected_args = helper.build_parser().parse_args(command[2:])
                record = mapping_identity(self.root.resolve(), task_uid, self.args.repo, self.issue_number,
                                          self.args.source_ref, self.args.target_ref)
                recovery = helper.PublicationRecoveryAuthority(selected_args, record, binding,
                                                               self.publication, publication, comments)
            try:
                command_output(command, timeout=180 if recovery is not None else 60,
                               reservation_fd=self.reservation_fd)
            except PublishInputError as exc:
                detail = str(exc)
                if "record-pr identity/vector conflict:" in detail:
                    raise publication.PublicationError(
                        "TASK_IDENTITY_CONFLICT", "record-pr rejected the live Issue/Project vector",
                    ) from exc
                if "record-pr publication-pending:" in detail:
                    reason = detail.split("record-pr publication-pending:", 1)[1].strip()
                    raise publication.PublicationError(
                        "NETWORK_UNCERTAIN", reason[:400] or "record-pr transition remains pending",
                    ) from exc
                raise
            if recovery is not None:
                # CLI success alone is not publication observation authority.
                # The core may observe H1 only after this separate four-surface
                # exact readback, including unique reciprocal binding content.
                recovery.check(final=True)
        finally:
            path.unlink(missing_ok=True)
        self.pr_number = number

    def find_task_publication_bindings(self, publication_id: str) -> dict[str, Any]:
        self._assert_task_identity()
        matches = []
        for comment in self._issue_comments():
            if "<!-- oasis7-ci-publication-binding/v1 -->" not in comment["body"]:
                continue
            binding = publication.parse_publication_binding_comment(comment["body"])
            if binding["publication_id"] == publication_id:
                matches.append(binding)
        return {"complete": True, "bindings": matches}

    def publish_reciprocal_binding(self, binding: dict[str, Any]) -> None:
        self._assert_task_identity()
        self._write_issue_comment(publication.publication_binding_comment(binding))

    def read_pr(self, repository: str, number: int) -> dict[str, Any]:
        item = json.loads(self.gh("api", f"repos/{repository}/pulls/{number}"))
        head = item.get("head") if isinstance(item.get("head"), dict) else {}
        base = item.get("base") if isinstance(item.get("base"), dict) else {}
        head_repo = head.get("repo") if isinstance(head.get("repo"), dict) else {}
        base_repo = base.get("repo") if isinstance(base.get("repo"), dict) else {}
        return {
            "repository": repository if head_repo.get("full_name") == base_repo.get("full_name") == repository else "",
            "number": item.get("number"), "source_ref": head.get("ref"),
            "target_ref": base.get("ref"), "head_oid": head.get("sha"),
            "body": item.get("body") or "", "state": str(item.get("state") or "").lower(),
            "merged": bool(item.get("merged_at")), "draft": item.get("draft"),
            "created_at": item.get("created_at"), "updated_at": item.get("updated_at"),
            "pr_author": (item.get("user") or {}).get("login") if isinstance(item.get("user"), dict) else None,
            "pr_author_type": (item.get("user") or {}).get("type") if isinstance(item.get("user"), dict) else None,
        }

    def patch_pr_body(self, repository: str, number: int, body: str) -> None:
        self.gh("api", f"repos/{repository}/pulls/{number}", "--method", "PATCH",
                "--input", "-", timeout=5.0, input_json={"body": body})


def _is_exact_create_retry(pr: dict[str, Any], publication_value: dict[str, Any],
                           projection_value: dict[str, Any]) -> bool:
    """Recognize an existing draft that exactly represents this create candidate."""
    if (not isinstance(pr, dict) or not isinstance(publication_value, dict)
            or not isinstance(projection_value, dict)
            or pr.get("repository") != publication_value.get("repository")
            or pr.get("source_ref") != publication_value.get("source_ref")
            or pr.get("target_ref") != publication_value.get("target_ref")
            or pr.get("head_oid") != publication_value.get("source_head_oid")
            or pr.get("state") != "open" or pr.get("merged") is not False
            or pr.get("draft") is not True
            or type(pr.get("number")) is not int or pr["number"] < 1
            or not isinstance(pr.get("body"), str)):
        return False

    consumed = projection_value.get("consumed_contracts")
    if not isinstance(consumed, list):
        return False
    try:
        clauses = [
            item if isinstance(item, str) else json.dumps(
                item, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
            )
            for item in consumed
        ]
        expected_contract, _marker = publication.prepare(
            task_uid=publication_value["task_uid"],
            source_head_oid=publication_value["source_head_oid"],
            scope_base_oid=publication_value["source_scope_oid"],
            projection_digest=publication_value["projection_digest"],
            clauses=clauses,
        )
        return decode_marker(pr["body"]) == expected_contract
    except (ContractError, KeyError, TypeError, ValueError):
        return False


def _prior_create_push_lease(journal: pr_projection_journal.PublicationJournal,
                             publication_value: dict[str, Any], *,
                             absent_lease: str | None = None) -> str | None:
    """Return the exact lease from an existing create push intent, if present."""
    action_id = "push:" + publication_value["publication_id"]
    with journal.locked():
        matches = [
            action for action in journal.read_action_state()["actions"]
            if isinstance(action, dict) and action.get("action_id") == action_id
        ]
        if len(matches) > 1:
            raise pr_projection_journal.JournalError("duplicate source push action")
        if not matches:
            return absent_lease
        expected = matches[0].get("expected")
        if (not isinstance(expected, dict)
                or set(expected) != {"source_ref", "new_oid", "lease_oid"}
                or expected.get("source_ref") != publication_value["source_ref"]
                or expected.get("new_oid") != publication_value["source_head_oid"]):
            raise pr_projection_journal.JournalError(
                "prior source push action conflicts with publication identity",
            )
        lease_oid = expected.get("lease_oid")
        if (lease_oid is not None
                and (not isinstance(lease_oid, str)
                     or re.fullmatch(r"[0-9a-f]{40,64}", lease_oid) is None)):
            raise pr_projection_journal.JournalError("prior source push lease is malformed")
        return lease_oid


def publish(args: argparse.Namespace) -> dict[str, Any]:
    root = Path(args.worktree).resolve(strict=True)
    candidate, projection_value = task_publication(root, args)
    adapter = GitHubPublicationAdapter(root, args, candidate)
    journal = pr_projection_journal.open_journal(
        repo_common_dir(root), candidate["repository"], candidate["source_ref"],
        candidate["publication_id"], task_uid=candidate["task_uid"],
        source_head_oid=candidate["source_head_oid"],
        scope_base_oid=candidate["source_scope_oid"],
        projection_digest=candidate["projection_digest"],
        canonical_worktree=root,
    )
    body = Path(args.body_file).read_text(encoding="utf-8")
    ready_update = bool(getattr(args, "existing_ready_update", False))
    if not has_exact_task_pr_linkage(body, args.task_uid, args.issue_number):
        raise PublishInputError(
            "PR body must contain exactly one canonical Task UID line and non-closing Refs line"
        )
    legacy_projection_b64 = base64.b64encode(Path(args.projection).read_bytes()).decode("ascii")
    visible = adapter.find_task_prs(candidate["task_uid"], candidate["source_ref"], candidate["target_ref"])
    if visible.get("complete") is not True or not isinstance(visible.get("pull_requests"), list):
        raise PublishInputError("PR discovery is incomplete")
    prs = visible["pull_requests"]
    if len(prs) > 1:
        raise PublishInputError("multiple PRs match the canonical Task branch")
    if ready_update and not prs:
        raise PublishInputError("existing-ready-update requires one existing PR; creation is forbidden")
    if prs:
        pr = prs[0]
        if not has_exact_task_pr_linkage(pr.get("body"), args.task_uid, args.issue_number):
            raise PublishInputError("existing PR body lacks exact unique Task/Refs identity")
        if not ready_update and _is_exact_create_retry(pr, candidate, projection_value):
            # A previous create may already have pushed H1 and created this
            # exact draft even if its final response or Task URL write was
            # lost. Re-enter the create reconciler so it can reuse the original
            # push intent (or skip it for a bound PR) instead of inventing an
            # H1 lease for the same publication action.
            prior_lease = _prior_create_push_lease(journal, candidate)
            result = publication.publish_create(
                adapter, journal, publication=candidate, projection=projection_value,
                body=pr["body"], expected_remote_oid=prior_lease,
                legacy_projection_b64=legacy_projection_b64,
                resume_action_id=getattr(args, "resume_action_id", None),
            )
        else:
            result = publication.publish_update(
                adapter, journal, publication=candidate, projection=projection_value,
                pr_number=pr["number"], old_head_oid=pr["head_oid"], body=body,
                legacy_projection_b64=legacy_projection_b64,
                expected_draft=not ready_update, existing_ready_update=ready_update,
                resume_action_id=getattr(args, "resume_action_id", None),
            )
    else:
        remote_oid = adapter.read_source_ref(candidate["source_ref"])
        # A lost push readback can leave H1 remote before any PR exists.
        # Preserve the original intent lease (including null); the live OID
        # is a lease only for a publication without a prior push intent.
        prior_lease = _prior_create_push_lease(journal, candidate, absent_lease=remote_oid)
        result = publication.publish_create(
            adapter, journal, publication=candidate, projection=projection_value,
            body=body, expected_remote_oid=prior_lease,
            legacy_projection_b64=legacy_projection_b64,
            resume_action_id=getattr(args, "resume_action_id", None),
        )
    result["pr_url"] = f"https://github.com/{candidate['repository']}/pull/{result['pr_number']}"
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--issue-number", required=True, type=int)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--remote", required=True)
    parser.add_argument("--source-ref", required=True)
    parser.add_argument("--target-ref", required=True)
    parser.add_argument("--source-head", required=True)
    parser.add_argument("--target-oid", required=True)
    parser.add_argument("--projection", required=True)
    parser.add_argument("--body-file", required=True)
    parser.add_argument("--task-helper", required=True)
    parser.add_argument("--title", default="")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--existing-ready-update", action="store_true")
    parser.add_argument("--resume-action-id", default=None,
                        help="resume only the exact persisted Task publication action")
    args = parser.parse_args()
    try:
        result = publish(args)
    except (OSError, ValueError, RuntimeError, PublishInputError, publication.PublicationError,
            pr_projection_journal.JournalError, subprocess.SubprocessError) as exc:
        print(f"pr-projection-publish: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, sort_keys=True) if args.json else result["pr_url"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
