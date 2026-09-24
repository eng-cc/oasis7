#!/usr/bin/env python3
"""Task-bound draft PR publication entrypoint for the C1 ordered publisher."""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
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


def command_output(args: list[str], *, timeout: float = LOCAL_COMMAND_TIMEOUT_SECONDS) -> str:
    try:
        return subprocess.run(args, check=True, text=True, encoding="utf-8",
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              timeout=timeout).stdout.strip()
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

    def gh(self, *args: str, timeout: float = 5.0,
           input_json: dict[str, Any] | None = None) -> str:
        input_text = json.dumps(input_json, ensure_ascii=False) if input_json is not None else None
        try:
            result = subprocess.run(["gh", *args], input=input_text, check=True, text=True,
                                    encoding="utf-8", stdout=subprocess.PIPE,
                                    stderr=subprocess.PIPE, timeout=timeout)
            return result.stdout.strip()
        except (OSError, subprocess.SubprocessError) as exc:
            raise RuntimeError(f"GitHub request failed: {exc}") from exc

    def _issue_comments(self) -> list[dict[str, Any]]:
        raw = self.gh("api", f"repos/{self.args.repo}/issues/{self.issue_number}/comments",
                      "--paginate", "--slurp", timeout=5.0)
        try:
            pages = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise RuntimeError("Task comments readback is malformed") from exc
        if not isinstance(pages, list):
            raise RuntimeError("Task comments readback is malformed")
        values = [item for page in pages for item in (page if isinstance(page, list) else [page])]
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
        if readback.get("body") != body:
            raise RuntimeError("Issue comment exact readback failed")

    def find_task_publications(self, publication_id: str) -> dict[str, Any]:
        self._assert_task_identity()
        target = self.publication
        matches = []
        for comment in self._issue_comments():
            body = comment["body"]
            if "<!-- oasis7-ci-publication/v1 -->" not in body:
                continue
            value = publication.parse_publication_comment(body)
            if (value["task_uid"] == target["task_uid"]
                    and value["source_head_oid"] == target["source_head_oid"]
                    and value["source_scope_oid"] == target["source_scope_oid"]):
                matches.append(value)
        return {"complete": True, "publications": matches}

    def publish_task_intent(self, value: dict[str, Any]) -> None:
        self._assert_task_identity()
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
                        f"{new_oid}:refs/heads/{source_ref}"], timeout=60)

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
            command_output(command, timeout=60)
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
        if number is not None:
            self.pr_number = number
        return {"task_uid": task_uid, "pr_number": number}

    def record_pr(self, task_uid: str, number: int, publication_id: str) -> None:
        url = f"https://github.com/{self.args.repo}/pull/{number}"
        binding = publication.build_publication_binding(self.publication, number, url)
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            json.dump(binding, handle, ensure_ascii=False, sort_keys=True)
            path = Path(handle.name)
        try:
            command_output([
            sys.executable, str(self.task_helper), "record-pr", str(self.root),
                "--repo", self.args.repo,
                "--task-uid", task_uid, "--pr-url", url, "--role", "tpm",
                "--validation-command", "C1 ordered CI projection publication",
                "--draft-candidate", "--publication-binding-json", str(path), "--json",
            ], timeout=60)
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
                             publication_value: dict[str, Any]) -> str | None:
    """Return the exact lease from an existing create push intent, if present."""
    action_id = "push:" + publication_value["publication_id"]
    with journal.locked():
        matches = [
            action for action in journal.read()["actions"]
            if isinstance(action, dict) and action.get("action_id") == action_id
        ]
        if len(matches) > 1:
            raise pr_projection_journal.JournalError("duplicate source push action")
        if not matches:
            return None
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
    )
    body = Path(args.body_file).read_text(encoding="utf-8")
    if (f"Task: {args.task_uid}" not in body or f"Refs #{args.issue_number}" not in body):
        raise PublishInputError("PR body must preserve the canonical Task UID and non-closing Refs line")
    legacy_projection_b64 = base64.b64encode(Path(args.projection).read_bytes()).decode("ascii")
    visible = adapter.find_task_prs(candidate["task_uid"], candidate["source_ref"], candidate["target_ref"])
    if visible.get("complete") is not True or not isinstance(visible.get("pull_requests"), list):
        raise PublishInputError("PR discovery is incomplete")
    prs = visible["pull_requests"]
    if len(prs) > 1:
        raise PublishInputError("multiple PRs match the canonical Task branch")
    if prs:
        pr = prs[0]
        if (f"Task: {args.task_uid}" not in pr["body"]
                or f"Refs #{args.issue_number}" not in pr["body"]):
            raise PublishInputError("existing PR body lost canonical Task identity")
        if _is_exact_create_retry(pr, candidate, projection_value):
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
            )
        else:
            result = publication.publish_update(
                adapter, journal, publication=candidate, projection=projection_value,
                pr_number=pr["number"], old_head_oid=pr["head_oid"], body=body,
                legacy_projection_b64=legacy_projection_b64,
            )
    else:
        remote_oid = adapter.read_source_ref(candidate["source_ref"])
        result = publication.publish_create(
            adapter, journal, publication=candidate, projection=projection_value,
            body=body, expected_remote_oid=remote_oid,
            legacy_projection_b64=legacy_projection_b64,
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
