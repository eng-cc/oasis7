#!/usr/bin/env python3
"""Independent read-only GitHub readback for V1 reuse validation.

No issue, comment, run, job, check, or artifact IDs are accepted as command
arguments. The live GitHub record set and the fixed Task/PR/workflow route are
the only source of those identities. This module never dispatches or reruns a
workflow and never changes production capability selection.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit
import zipfile
from io import BytesIO
from typing import Any, Mapping


REPOSITORY = "eng-cc/oasis7"
TASK_ISSUE_NUMBER = 4059
PR_NUMBER = 4060
WORKFLOW_FILE = ".github/workflows/rust.yml"
WORKFLOW_PATH = WORKFLOW_FILE + "@main"
WORKFLOW_REF = f"{REPOSITORY}/{WORKFLOW_FILE}@refs/heads/main"
API_ORIGIN = "https://api.github.com"
PAGE_SIZE = 100
MAX_ARCHIVE_BYTES = 16 * 1024 * 1024
MAX_PAYLOAD_BYTES = 1024 * 1024
_TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40}\Z")


class ReadbackError(ValueError):
    """Live readback was incomplete, ambiguous, or inconsistent."""


def _load_contract():
    path = Path(__file__).with_name("ci_reuse_validation_contract.py")
    if not path.is_file() or path.is_symlink():
        raise ReadbackError("validation-only contract helper is unavailable")
    spec = importlib.util.spec_from_file_location("ci_reuse_validation_contract", path)
    if spec is None or spec.loader is None:
        raise ReadbackError("validation-only contract helper cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except (OSError, ImportError, ValueError) as exc:
        raise ReadbackError("validation-only contract helper failed to load") from exc
    return module


contract = _load_contract()


def _decode_included_response(raw: bytes) -> tuple[dict[str, str], bytes]:
    """Decode one successful HTTP response emitted by ``gh api --include``."""
    if not isinstance(raw, bytes):
        raise ReadbackError("GitHub API response is not bytes")
    normalized = raw.replace(b"\r\n", b"\n")
    split = normalized.find(b"\n\n")
    if split < 0:
        raise ReadbackError("GitHub API response headers are missing")
    header_bytes, body = normalized[:split], normalized[split + 2:]
    try:
        lines = header_bytes.decode("utf-8", "strict").split("\n")
    except UnicodeDecodeError as exc:
        raise ReadbackError("GitHub API response headers are malformed") from exc
    if not lines or not re.fullmatch(r"HTTP/\S+ 2\d\d(?: .*)?", lines[0]):
        raise ReadbackError("GitHub API response status is not successful")
    headers: dict[str, str] = {}
    for line in lines[1:]:
        if not line:
            continue
        if ":" not in line:
            raise ReadbackError("GitHub API response header is malformed")
        name, value = line.split(":", 1)
        key = name.strip().lower()
        if not key or key in headers:
            raise ReadbackError("GitHub API response has duplicate or empty headers")
        headers[key] = value.strip()
    return headers, body


def _next_link(
    link_header: str | None, *, allowed_paths: frozenset[str], current_url: str,
) -> str | None:
    if link_header is None:
        return None
    next_links: list[str] = []
    for part in link_header.split(","):
        match = re.fullmatch(r'\s*<([^<>]+)>\s*;\s*rel="?([A-Za-z0-9_-]+)"?\s*', part)
        if not match:
            raise ReadbackError("GitHub pagination Link header is malformed")
        if match.group(2) == "next":
            next_links.append(match.group(1))
    if len(next_links) > 1:
        raise ReadbackError("GitHub pagination has multiple next links")
    if not next_links:
        return None
    candidate = next_links[0]
    parsed, current = urlsplit(candidate), urlsplit(current_url)
    if (current.scheme != "https" or current.netloc != "api.github.com"
            or current.path not in allowed_paths or current.fragment
            or parsed.scheme != "https" or parsed.netloc != "api.github.com"
            or parsed.path not in allowed_paths or parsed.fragment):
        raise ReadbackError("GitHub pagination next link leaves the canonical API endpoint")
    try:
        from urllib.parse import parse_qs
        params = parse_qs(parsed.query, keep_blank_values=True, strict_parsing=True)
        current_params = parse_qs(current.query, keep_blank_values=True, strict_parsing=True)
    except ValueError as exc:
        raise ReadbackError("GitHub pagination query is malformed") from exc
    if (set(params) != {"per_page", "page"}
            or params.get("per_page") != [str(PAGE_SIZE)]
            or len(params.get("page", [])) != 1
            or not re.fullmatch(r"[1-9][0-9]*", params["page"][0])):
        raise ReadbackError("GitHub pagination next link changes the unfiltered page query")
    old_pages = current_params.get("page", ["1"])
    if (set(current_params) not in ({"per_page"}, {"per_page", "page"})
            or current_params.get("per_page") != [str(PAGE_SIZE)]
            or len(old_pages) != 1
            or not re.fullmatch(r"[1-9][0-9]*", old_pages[0])):
        raise ReadbackError("GitHub pagination current page is malformed")
    expected_page = int(old_pages[0]) + 1
    if (int(params["page"][0]) != expected_page
            or parsed.query != f"per_page={PAGE_SIZE}&page={expected_page}"):
        raise ReadbackError("GitHub pagination next link is not the following page")
    return parsed.path + "?" + parsed.query


class GitHubReadOnly:
    """Small REST client that follows every live pagination relation."""

    def _included_json(self, endpoint: str) -> tuple[dict[str, str], Any]:
        try:
            raw = subprocess.check_output(["gh", "api", "--include", endpoint])
        except (OSError, subprocess.CalledProcessError) as exc:
            raise ReadbackError(f"GitHub API read failed for {endpoint}") from exc
        headers, body = _decode_included_response(raw)
        try:
            value = json.loads(body.decode("utf-8", "strict"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ReadbackError(f"GitHub JSON response is malformed for {endpoint}") from exc
        return headers, value

    def get_json(self, endpoint: str) -> Any:
        _, value = self._included_json(endpoint)
        return value

    def get_bytes(self, endpoint: str) -> bytes:
        try:
            value = subprocess.check_output(["gh", "api", endpoint])
        except (OSError, subprocess.CalledProcessError) as exc:
            raise ReadbackError("GitHub artifact download failed") from exc
        if type(value) is not bytes:
            raise ReadbackError("GitHub artifact download returned non-byte content")
        return value

    def _paginated_observations(
        self, endpoint: str, *, collection_key: str | None,
        allowed_paths: frozenset[str] | None = None,
    ) -> tuple[tuple[Any, bool], ...]:
        parsed = urlsplit(endpoint)
        if parsed.scheme or parsed.netloc or parsed.fragment or not parsed.path.startswith("repos/"):
            raise ReadbackError("GitHub pagination endpoint is not a repository-relative path")
        origin_path = "/" + parsed.path
        accepted_paths = frozenset({origin_path}) if allowed_paths is None else allowed_paths
        if origin_path not in accepted_paths or any(not path.startswith("/") for path in accepted_paths):
            raise ReadbackError("GitHub pagination endpoint is outside its authenticated resource paths")
        current_url = API_ORIGIN + origin_path + ("?" + parsed.query if parsed.query else "")
        current_endpoint = endpoint
        visited: set[str] = set()
        pages: list[tuple[Any, bool]] = []
        while True:
            if current_url in visited:
                raise ReadbackError("GitHub pagination cycle detected")
            visited.add(current_url)
            headers, value = self._included_json(current_endpoint)
            if collection_key is None:
                if type(value) is not list:
                    raise ReadbackError("GitHub paginated array is malformed")
            elif not isinstance(value, dict) or type(value.get(collection_key)) is not list:
                raise ReadbackError("GitHub paginated collection is malformed")
            following = _next_link(
                headers.get("link"), allowed_paths=accepted_paths, current_url=current_url,
            )
            pages.append((value, following is not None))
            if following is None:
                return tuple(pages)
            current_endpoint = following.lstrip("/")
            current_url = API_ORIGIN + following

    def paginated_pages(self, endpoint: str, *, collection_key: str | None) -> tuple[Any, ...]:
        return tuple(value for value, _ in self._paginated_observations(
            endpoint, collection_key=collection_key,
        ))

    def paginated_collections(self, endpoint: str, *, collection_key: str) -> tuple[dict[str, Any], ...]:
        observations = self._paginated_observations(endpoint, collection_key=collection_key)
        result: list[dict[str, Any]] = []
        for value, has_next in observations:
            if type(value.get("total_count")) is not int:
                raise ReadbackError(f"GitHub {collection_key} total_count is missing or malformed")
            result.append({"total_count": value["total_count"],
                           collection_key: value[collection_key], "has_next": has_next})
        return tuple(result)

    def issue_comment_pages(self) -> tuple[Any, ...]:
        return self.paginated_pages(
            f"repos/{REPOSITORY}/issues/{TASK_ISSUE_NUMBER}/comments?per_page={PAGE_SIZE}",
            collection_key=None,
        )

    def workflow_run_pages(
        self, workflow_id: int, repository_id: int | None = None,
    ) -> tuple[dict[str, Any], ...]:
        if type(workflow_id) is not int or workflow_id <= 0:
            raise ReadbackError("live workflow ID is invalid")
        if (repository_id is not None
                and (type(repository_id) is not int or repository_id <= 0)):
            raise ReadbackError("live repository ID is invalid")
        live_workflow_id, _default_branch, live_repository_id = _live_workflow(self)
        if workflow_id != live_workflow_id:
            raise ReadbackError("workflow-run pagination ID differs from live rust.yml")
        if repository_id is not None and repository_id != live_repository_id:
            raise ReadbackError("workflow-run pagination repository ID differs from live repository")
        repository_id = live_repository_id
        # Intentionally no filter query: filtered Actions searches are capped.
        workflow_paths = frozenset({
            f"/repos/{REPOSITORY}/actions/workflows/{workflow_id}/runs",
            f"/repositories/{repository_id}/actions/workflows/{workflow_id}/runs",
        })
        observations = self._paginated_observations(
            f"repos/{REPOSITORY}/actions/workflows/{workflow_id}/runs?per_page={PAGE_SIZE}",
            collection_key="workflow_runs",
            allowed_paths=workflow_paths,
        )
        pages: list[dict[str, Any]] = []
        for page, has_next in observations:
            if type(page.get("total_count")) is not int:
                raise ReadbackError("workflow-run total_count is missing or malformed")
            pages.append({"total_count": page["total_count"],
                          "runs": page["workflow_runs"], "has_next": has_next})
        return tuple(pages)

    def workflow_pages(self) -> tuple[Any, ...]:
        return self.paginated_pages(
            f"repos/{REPOSITORY}/actions/workflows?per_page={PAGE_SIZE}",
            collection_key="workflows",
        )

    def job_pages(self, run_id: int, attempt: int) -> tuple[Any, ...]:
        return self.paginated_collections(
            f"repos/{REPOSITORY}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page={PAGE_SIZE}",
            collection_key="jobs",
        )

    def artifact_pages(self, run_id: int) -> tuple[Any, ...]:
        return self.paginated_collections(
            f"repos/{REPOSITORY}/actions/runs/{run_id}/artifacts?per_page={PAGE_SIZE}",
            collection_key="artifacts",
        )


def _flatten_array_pages(pages: tuple[Any, ...], label: str) -> tuple[Mapping[str, Any], ...]:
    if not pages:
        raise ReadbackError(f"complete {label} listing is empty or unavailable")
    rows: list[Mapping[str, Any]] = []
    for page in pages:
        if type(page) is not list:
            raise ReadbackError(f"{label} listing page is malformed")
        rows.extend(page)
    if any(not isinstance(row, Mapping) for row in rows):
        raise ReadbackError(f"{label} listing contains malformed rows")
    return tuple(rows)


def _collection_rows(pages: tuple[Any, ...], key: str, label: str) -> tuple[Mapping[str, Any], ...]:
    if not pages:
        raise ReadbackError(f"complete {label} listing is unavailable")
    expected_total: int | None = None
    rows: list[Mapping[str, Any]] = []
    seen: set[int] = set()
    for index, page in enumerate(pages):
        if not isinstance(page, Mapping) or type(page.get("total_count")) is not int:
            raise ReadbackError(f"{label} total_count is missing or malformed")
        total = page["total_count"]
        if total < 0 or (expected_total is not None and total != expected_total):
            raise ReadbackError(f"{label} total_count changed during pagination")
        expected_total = total
        batch = page.get(key)
        if type(batch) is not list:
            raise ReadbackError(f"{label} collection is malformed")
        for row in batch:
            if not isinstance(row, Mapping) or type(row.get("id")) is not int or row["id"] <= 0:
                raise ReadbackError(f"{label} row identity is malformed")
            if row["id"] in seen:
                raise ReadbackError(f"{label} listing contains duplicate IDs")
            seen.add(row["id"])
            rows.append(row)
        has_next = page.get("has_next")
        if type(has_next) is not bool or has_next != (index < len(pages) - 1):
            raise ReadbackError(f"{label} pagination is incomplete or uncertain")
    if expected_total != len(seen):
        raise ReadbackError(f"{label} total_count differs from the complete unique row set")
    return tuple(rows)


def _live_task_uid(issue: Mapping[str, Any]) -> str:
    if issue.get("number") != TASK_ISSUE_NUMBER or type(issue.get("body")) is not str:
        raise ReadbackError("live validation Task Issue identity is unavailable")
    matches = re.findall(r"(?m)^task_uid: (task_[0-9a-f]{32})\s*$", issue["body"])
    if len(matches) != 1 or not _TASK_UID_RE.fullmatch(matches[0]):
        raise ReadbackError("Task Issue does not contain one canonical Task UID")
    return matches[0]


def _check_live_pr(pr: Mapping[str, Any], task_uid: str) -> tuple[str, str]:
    if (pr.get("number") != PR_NUMBER or pr.get("state") != "open" or pr.get("merged") is not False
            or pr.get("base", {}).get("repo", {}).get("full_name") != REPOSITORY
            or pr.get("head", {}).get("repo", {}).get("full_name") != REPOSITORY
            or pr.get("base", {}).get("ref") != "main"):
        raise ReadbackError("live reciprocal PR is closed, merged, cross-repository, or targets another branch")
    head = pr.get("head", {}).get("sha")
    base = pr.get("base", {}).get("sha")
    if not isinstance(head, str) or not _OID_RE.fullmatch(head) or not isinstance(base, str) or not _OID_RE.fullmatch(base):
        raise ReadbackError("live PR H/B identity is malformed")
    body = pr.get("body")
    if type(body) is not str or body.count(f"Task: {task_uid}") != 1:
        raise ReadbackError("live PR does not reciprocally bind the canonical Task UID")
    if body.count(f"Refs #{TASK_ISSUE_NUMBER}") != 1:
        raise ReadbackError("live PR does not reciprocally reference the fixed Task Issue")
    return head, base


def _resolve_records(comment_pages: tuple[Any, ...]):
    comments = _flatten_array_pages(comment_pages, "Issue comment")
    authority = contract.resolve_records_for_readback(comments)
    return comments, authority


def _live_workflow(api: GitHubReadOnly) -> tuple[int, str, int]:
    repo = api.get_json(f"repos/{REPOSITORY}")
    owner = repo.get("owner") if isinstance(repo, Mapping) else None
    repository_id = repo.get("id") if isinstance(repo, Mapping) else None
    if (not isinstance(repo, Mapping) or repo.get("full_name") != REPOSITORY
            or repo.get("name") != "oasis7" or not isinstance(owner, Mapping)
            or owner.get("login") != "eng-cc" or type(repository_id) is not int
            or repository_id <= 0 or repo.get("default_branch") != "main"):
        raise ReadbackError("canonical repository identity or default branch is invalid")
    workflow = api.get_json(f"repos/{REPOSITORY}/actions/workflows/rust.yml")
    if (not isinstance(workflow, Mapping) or workflow.get("path") != WORKFLOW_FILE
            or type(workflow.get("id")) is not int or workflow["id"] <= 0):
        raise ReadbackError("canonical rust.yml workflow ID is missing or ambiguous")
    if workflow.get("state") != "active":
        raise ReadbackError("canonical rust.yml workflow is not active")
    return workflow["id"], repo["default_branch"], repository_id


def _run_identity(row: Mapping[str, Any], workflow_id: int, default_branch: str) -> dict[str, Any]:
    run_id = row.get("id")
    attempt = row.get("run_attempt")
    sha = row.get("head_sha")
    path = row.get("path")
    branch = row.get("head_branch")
    head_repo = row.get("head_repository")
    repository = row.get("repository")
    if (type(run_id) is not int or run_id <= 0 or type(attempt) is not int or attempt <= 0
            or type(sha) is not str or not _OID_RE.fullmatch(sha)
            or path != WORKFLOW_PATH or branch != default_branch
            or not isinstance(head_repo, Mapping) or head_repo.get("full_name") != REPOSITORY
            or not isinstance(repository, Mapping) or repository.get("full_name") != REPOSITORY
            or row.get("workflow_id") != workflow_id):
        raise ReadbackError("live workflow run provenance is incomplete or differs from canonical W")
    return {
        "repository": REPOSITORY,
        "id": run_id,
        "workflow_id": workflow_id,
        "workflow_path": path,
        "workflow_ref": f"{REPOSITORY}/{WORKFLOW_FILE}@refs/heads/{default_branch}",
        "workflow_sha": sha,
        "event": row.get("event"),
        "display_title": row.get("display_title"),
        "dispatched_head_sha": sha,
        "run_attempt": attempt,
        "created_at": row.get("created_at"),
        "head_branch": branch,
        "head_repository": head_repo,
        "status": row.get("status"),
        "conclusion": row.get("conclusion"),
    }


def _read_live_run(api: GitHubReadOnly, row: Mapping[str, Any], workflow_id: int,
                   default_branch: str) -> dict[str, Any]:
    run_id = row.get("id")
    if type(run_id) is not int or run_id <= 0:
        raise ReadbackError("selected workflow run ID is invalid")
    response = api.get_json(f"repos/{REPOSITORY}/actions/runs/{run_id}")
    if not isinstance(response, Mapping) or response.get("id") != run_id:
        raise ReadbackError("selected workflow run live readback has another identity")
    current = _run_identity(response, workflow_id, default_branch)
    if response.get("display_title") != row.get("display_title"):
        raise ReadbackError("selected workflow run title changed between discovery and direct read")
    return current


def _latest_validation_check(api: GitHubReadOnly, run: Mapping[str, Any]) -> dict[str, Any]:
    jobs = _collection_rows(
        api.job_pages(run["id"], run["run_attempt"]), "jobs", "attempt jobs",
    )
    eligible: list[Mapping[str, Any]] = []
    for job in jobs:
        if (job.get("run_id") != run["id"] or job.get("run_attempt") != run["run_attempt"]
                or job.get("head_sha") != run["dispatched_head_sha"]
                or type(job.get("id")) is not int or job["id"] <= 0
                or type(job.get("name")) is not str):
            raise ReadbackError("attempt job does not bind exact live R/A/head identity")
        if job["name"] == contract.CHECK_NAME:
            eligible.append(job)
    if len(eligible) != 1:
        raise ReadbackError("exact validation-only job is missing or ambiguous for latest R/A")
    job = eligible[0]
    parsed = urlsplit(str(job.get("check_run_url") or ""))
    check_match = re.fullmatch(rf"/repos/{re.escape(REPOSITORY)}/check-runs/([1-9][0-9]*)", parsed.path)
    if parsed.scheme != "https" or parsed.netloc != "api.github.com" or check_match is None or parsed.query or parsed.fragment:
        raise ReadbackError("validation job check-run URL is not canonical")
    check_id = int(check_match.group(1))
    check = api.get_json(f"repos/{REPOSITORY}/check-runs/{check_id}")
    app = check.get("app") if isinstance(check, Mapping) else None
    if not isinstance(check, Mapping) or type(app) is not dict:
        raise ReadbackError("live validation check-run response is malformed")
    check_value = {
        "id": check.get("id"),
        "name": check.get("name"),
        "app_id": app.get("id"),
        "head_sha": check.get("head_sha"),
        "run_id": run["id"],
        "run_attempt": run["run_attempt"],
        "status": check.get("status"),
        "conclusion": check.get("conclusion"),
    }
    if check_value["id"] != check_id:
        raise ReadbackError("live check-run ID differs from exact job link")
    return check_value


def _exact_artifact(api: GitHubReadOnly, run: Mapping[str, Any], authority: Any) -> Mapping[str, Any]:
    rows = _collection_rows(api.artifact_pages(run["id"]), "artifacts", "run artifacts")
    expected_name = contract.artifact_name(authority.validation_id, run["id"], run["run_attempt"])
    attempt_prefix = f"oasis7-ci-reuse-validation-v1-{authority.validation_id}-r{run['id']}-a"
    candidates: list[Mapping[str, Any]] = []
    for item in rows:
        name = item.get("name")
        if type(name) is not str:
            raise ReadbackError("live artifact name is malformed")
        if name == expected_name:
            candidates.append(item)
            continue
        # A malformed alias for this same numeric attempt is a competing
        # validation payload; a legitimate artifact from an older/newer A is
        # not a duplicate of the latest attempt.
        if name.startswith(attempt_prefix):
            suffix = name[len(attempt_prefix):]
            digits = re.match(r"[0-9]+", suffix)
            if digits and int(digits.group(0)) == run["run_attempt"]:
                candidates.append(item)
    if len(candidates) != 1 or candidates[0].get("name") != expected_name:
        raise ReadbackError("exact validation artifact is missing, duplicated, or has a sibling payload")
    artifact = candidates[0]
    workflow_run = artifact.get("workflow_run")
    if (artifact.get("expired") is not False or type(artifact.get("id")) is not int
            or artifact["id"] <= 0 or not isinstance(workflow_run, Mapping)
            or workflow_run.get("id") != run["id"]
            or workflow_run.get("head_sha") != run["dispatched_head_sha"]):
        raise ReadbackError("live artifact is expired or bound to another workflow run")
    return artifact


def _payload_from_archive(archive_bytes: bytes) -> bytes:
    if type(archive_bytes) is not bytes or not archive_bytes or len(archive_bytes) > MAX_ARCHIVE_BYTES:
        raise ReadbackError("validation artifact archive is empty or exceeds its byte limit")
    try:
        with zipfile.ZipFile(BytesIO(archive_bytes), "r") as archive:
            members = archive.infolist()
            if (len(members) != 1 or members[0].filename != contract.PAYLOAD_MEMBER
                    or members[0].is_dir() or members[0].file_size <= 0
                    or members[0].file_size > MAX_PAYLOAD_BYTES or members[0].flag_bits & 1):
                raise ReadbackError("validation artifact ZIP must contain exactly the fixed payload member")
            with archive.open(members[0], "r") as member:
                payload = member.read(MAX_PAYLOAD_BYTES + 1)
                if len(payload) > MAX_PAYLOAD_BYTES or member.read(1):
                    raise ReadbackError("validation payload member exceeds its byte limit")
            if len(payload) != members[0].file_size:
                raise ReadbackError("validation payload member length differs from ZIP metadata")
    except (OSError, zipfile.BadZipFile, RuntimeError) as exc:
        raise ReadbackError("validation artifact ZIP is unreadable") from exc
    if not payload or len(payload) > MAX_PAYLOAD_BYTES:
        raise ReadbackError("validation payload member is empty or oversized")
    return payload


def _projection_digest_from_v2(body: str, expected: Mapping[str, Any], pr: Mapping[str, Any],
                               issue_comments: tuple[Mapping[str, Any], ...],
                               trusted_module_root: Path) -> str:
    """Resolve the live v2 PR projection through the exact-W resolver modules."""
    marker = "<!-- oasis7-ci-publication/v1 -->"
    binding_marker = "<!-- oasis7-ci-publication-binding/v1 -->"
    publications = [item["body"] for item in issue_comments
                    if type(item.get("body")) is str and marker in item["body"]]
    bindings = [item["body"] for item in issue_comments
                if type(item.get("body")) is str and binding_marker in item["body"]]
    if len(publications) != 1 or len(bindings) != 1:
        raise ReadbackError("Task Issue must have one immutable CI publication and reciprocal binding")
    try:
        projection_contract = _load_from_root(trusted_module_root, "projection_publication_contract")
        _load_from_root(trusted_module_root, "pr_projection_journal")
        publication_module = _load_from_root(trusted_module_root, "pr_projection_publication")
        resolver = _load_from_root(trusted_module_root, "pr_projection_resolver")
        publication = publication_module.parse_publication_comment(publications[0])
        binding = publication_module.parse_publication_binding_comment(bindings[0])
        result = resolver.resolve(
            body, task_uid=expected["task_uid"], source_head_oid=expected["head_oid"],
            scope_base_oid=expected["source_scope_oid"], publication=publication,
            binding=binding, repository=REPOSITORY, pr_number=PR_NUMBER,
            planner_config_sha256=publication["planner_config_sha256"],
            required_protocol="v2",
            live={"repository": REPOSITORY, "pr_number": PR_NUMBER,
                  "repository_id": pr.get("base", {}).get("repo", {}).get("id"),
                  "source_repository_id": pr.get("head", {}).get("repo", {}).get("id"),
                  "head_oid": pr.get("head", {}).get("sha"),
                  "state": pr.get("state"), "merged": pr.get("merged")},
        )
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise ReadbackError("live v2 Task/PR projection publication could not be verified") from exc
    digest = result.get("projection_digest") if isinstance(result, Mapping) else None
    if type(digest) is not str or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise ReadbackError("verified v2 PR projection digest is malformed")
    return digest


def _load_from_root(root: Path, name: str):
    path = root / "scripts" / "pm" / f"{name}.py"
    if not path.is_file() or path.is_symlink():
        raise ReadbackError(f"exact-W projection helper is unavailable: {name}")
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ReadbackError(f"exact-W projection helper cannot be loaded: {name}")
    module = importlib.util.module_from_spec(spec)
    # These names are the exact-W helpers' own import names. Replacing any
    # already-loaded local version prevents a newer checkout from satisfying
    # imports for an older trusted workflow SHA.
    sys.modules[spec.name] = module
    # W helper imports are adjacent and dependency-free or in scripts/pm.
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(module)
    except (OSError, ImportError, ValueError) as exc:
        raise ReadbackError(f"exact-W helper failed to load: {name}") from exc
    finally:
        sys.path.pop(0)
    return module


def _git(root: Path, *args: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), *args], stderr=subprocess.PIPE, text=True,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ReadbackError("exact W/B/H/M/T Git observation failed") from exc


def _trusted_inventory(context: Mapping[str, Any], run: Mapping[str, Any], check: Mapping[str, Any],
                       pr: Mapping[str, Any], issue_comments: tuple[Mapping[str, Any], ...],
                       workflow_sha: str, default_branch: str) -> tuple[Any, str, str]:
    """Recompute exact-W M/T and the complete dispatch inventory, without tests."""
    task_uid = context["task_uid"]
    base, head = context["integration_base_oid"], context["head_oid"]
    source_scope, projection_digest = context["source_scope_oid"], context["projection_digest"]
    try:
        with tempfile.TemporaryDirectory(prefix="oasis7-ci-reuse-readback-") as temp:
            repo_root = Path(temp) / "repo"
            try:
                subprocess.run(["gh", "repo", "clone", REPOSITORY, str(repo_root)],
                               check=True, capture_output=True, text=True)
                subprocess.run([
                    "git", "-C", str(repo_root), "fetch", "--no-tags", "origin",
                    f"+refs/heads/{default_branch}:refs/remotes/origin/{default_branch}",
                    f"+refs/pull/{PR_NUMBER}/head:refs/remotes/origin/pr-{PR_NUMBER}",
                ], check=True, capture_output=True, text=True)
            except (OSError, subprocess.CalledProcessError) as exc:
                raise ReadbackError("exact W/B/H objects cannot be fetched to a disposable repository") from exc
            if _git(repo_root, "rev-parse", "--verify", f"{head}^{{commit}}") != head:
                raise ReadbackError("live PR head object is unavailable in disposable repository")
            if _git(repo_root, "rev-parse", "--verify", f"{base}^{{commit}}") != base:
                raise ReadbackError("frozen integration base object is unavailable in disposable repository")
            if _git(repo_root, "rev-parse", "--verify", f"{workflow_sha}^{{commit}}") != workflow_sha:
                raise ReadbackError("exact workflow source commit is unavailable")
            try:
                subprocess.run(["git", "-C", str(repo_root), "merge-base", "--is-ancestor",
                                workflow_sha, f"refs/remotes/origin/{default_branch}"],
                               check=True, capture_output=True)
            except (OSError, subprocess.CalledProcessError) as exc:
                raise ReadbackError("selected workflow SHA is not in the live default-branch history") from exc
            w_root = Path(temp) / "planner-w"
            try:
                subprocess.run(["git", "-C", str(repo_root), "worktree", "add", "--detach",
                                str(w_root), workflow_sha], check=True, capture_output=True, text=True)
            except (OSError, subprocess.CalledProcessError) as exc:
                raise ReadbackError("exact trusted workflow checkout could not be created") from exc
            integration = _load_from_root(w_root, "integration_ci")
            projection = _projection_digest_from_v2(pr.get("body"), context, pr,
                                                   issue_comments, w_root)
            if projection != projection_digest:
                raise ReadbackError("live v2 projection digest differs from frozen request")
            scope = _git(repo_root, "merge-base", "--all", base, head).splitlines()
            if scope != [source_scope]:
                raise ReadbackError("exact B/H merge-base differs from frozen S")
            m_root = Path(temp) / "target-m"
            composition = integration.compose(w_root, base, head, str(m_root))
            merge_oid, tree_oid = composition.get("tested_commit_oid"), composition.get("tested_tree_oid")
            if (type(merge_oid) is not str or not _OID_RE.fullmatch(merge_oid)
                    or type(tree_oid) is not str or not _OID_RE.fullmatch(tree_oid)
                    or _git(m_root, "rev-parse", "HEAD") != merge_oid
                    or _git(m_root, "show", "-s", "--format=%T", "HEAD") != tree_oid):
                raise ReadbackError("exact W compose did not produce a verified M/T worktree")
            changed_paths = _git(repo_root, "diff", "--name-only", f"{source_scope}..{head}").splitlines()
            if changed_paths != sorted(set(changed_paths)):
                raise ReadbackError("complete B/H changed paths are noncanonical")
            projection_contract = _load_from_root(w_root, "projection_publication_contract")
            body_contract = projection_contract.decode_marker(pr.get("body"))
            if (body_contract.get("task_uid") != task_uid or body_contract.get("source_head_oid") != head
                    or body_contract.get("scope_base_oid") != source_scope
                    or body_contract.get("projection_digest") != projection_digest):
                raise ReadbackError("exact W v2 contract differs from task/H/S/D")
            inventory = _load_from_root(w_root, "ci_required_inventory")
            planner_path = w_root / "scripts" / "plan-rust-required-scope.py"
            config_path = w_root / "scripts" / "ci-required-scope.v2.json"
            # The exact W full inventory route is deliberately run without an
            # impact_projection object: the v2 publication contract carries D,
            # while dispatch's legacy planner mode enumerates the complete unit
            # universe. D is independently bound above.
            command = [sys.executable, str(planner_path), "--event-name", "workflow_dispatch",
                       "--run-mode", "legacy", "--config", str(config_path),
                       "--base-ref", base, "--head-ref", head,
                       "--task-uid", task_uid, "--scope-base-oid", source_scope]
            for path in changed_paths:
                command.extend(("--changed-path", path))
            try:
                planner_output = subprocess.run(command, cwd=w_root, check=True,
                                                capture_output=True, text=True).stdout
            except (OSError, subprocess.CalledProcessError) as exc:
                raise ReadbackError("exact-W full validation inventory planner replay failed") from exc
            try:
                result = inventory.build_required_inventory(
                    w_root, m_root, merge_oid, planner_output,
                    repository=REPOSITORY, workflow_ref=WORKFLOW_REF,
                    planner_authority_oid=workflow_sha, event_name="workflow_dispatch",
                    run_mode="legacy", changed_paths=changed_paths,
                    base_ref=base, head_ref=head, task_uid=task_uid,
                    scope_base_oid=source_scope, impact_projection=None,
                    run_id=run["id"], run_attempt=run["run_attempt"],
                    check_app_id=check["app_id"], check_run_id=check["id"],
                )
            except (OSError, ValueError, KeyError, TypeError) as exc:
                raise ReadbackError("exact-W complete validation inventory could not be rebuilt") from exc
            specs = result.get("unit_specs") if isinstance(result, Mapping) else None
            ids = result.get("planner_inventory_issuer", {}).get("unit_ids") if isinstance(result, Mapping) else None
            if (type(ids) is not list or not ids or ids != sorted(set(ids))
                    or type(specs) is not list):
                raise ReadbackError("exact-W planner inventory unit IDs are missing or noncanonical")
            obligations: dict[str, tuple[str, ...]] = {}
            for spec in specs:
                if not isinstance(spec, Mapping) or spec.get("unit_id") not in ids:
                    raise ReadbackError("exact-W inventory has an unknown or malformed unit spec")
                unit = spec["unit_id"]
                values = spec.get("obligation_set")
                if type(values) is not list or not values or any(type(item) is not str for item in values):
                    raise ReadbackError(f"exact-W inventory obligations are incomplete for {unit}")
                obligations[unit] = tuple(values)
            if set(obligations) != set(ids):
                raise ReadbackError("exact-W obligation sets do not cover the complete inventory")
            trusted = contract.TrustedRequestContext(
                task_uid=task_uid, head_oid=head, source_scope_oid=source_scope,
                projection_digest=projection_digest, planner_unit_ids=tuple(ids),
                planner_unit_obligations=obligations,
            )
            return trusted, merge_oid, tree_oid
    except ReadbackError:
        raise
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as exc:
        raise ReadbackError("exact-W independent inventory recomputation failed") from exc


def _compare_issue_authority(left: Any, right: Any) -> None:
    fields = (
        "validation_id", "request_comment_id", "authorization_comment_id", "pin_comment_id",
        "request_body_digest", "authorization_body_digest", "pin_body_digest",
        "authorized_actor", "pin_actor", "comment_timestamps",
    )
    for field in fields:
        if getattr(left, field, None) != getattr(right, field, None):
            raise ReadbackError(f"Issue validation authority changed at {field}")
    for field in ("request", "authorization", "pin"):
        if dict(getattr(left, field)) != dict(getattr(right, field)):
            raise ReadbackError(f"Issue validation {field} record changed during readback")


def _compare_authority(left: Any, right: Any) -> None:
    _compare_issue_authority(left, right)
    for field in ("approval_permission", "pin_permission", "permission_snapshot_bound"):
        if getattr(left, field, None) != getattr(right, field, None):
            raise ReadbackError(f"recorded permission snapshot changed at {field}")


_AUTHORITY_RECORD_FIELDS = (
    "schema", "repository", "capability_under_test", "validation_id", "task_uid",
    "task_issue_number", "pr_number", "head_oid", "integration_base_oid",
    "source_scope_oid", "projection_digest", "validation_units", "purpose",
    "request_comment_id", "request_body_digest", "request_digest",
    "authorization_comment_id", "authorization_body_digest", "pin_comment_id",
    "pin_body_digest", "authorized_actor", "pin_actor", "approval_permission",
    "pin_permission", "run_id", "workflow_id", "workflow_path", "workflow_ref",
    "workflow_sha", "event", "display_title", "dispatched_head_sha",
)


def _authority_record_from_payload(payload: Mapping[str, Any]) -> dict[str, Any]:
    if not isinstance(payload, Mapping):
        raise ReadbackError("downloaded validation payload is not an object")
    try:
        record = {key: payload[key] for key in _AUTHORITY_RECORD_FIELDS}
    except KeyError as exc:
        raise ReadbackError("downloaded payload lacks its complete authority record") from exc
    record["schema"] = contract.AUTHORITY_SCHEMA
    return record


def _parse_payload_bytes(payload_bytes: bytes) -> dict[str, Any]:
    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            if key in value:
                raise ReadbackError("canonical validation payload contains a duplicate key")
            value[key] = item
        return value

    try:
        payload = json.loads(
            payload_bytes.decode("utf-8", "strict"), object_pairs_hook=reject_duplicate_keys,
        )
    except ReadbackError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ReadbackError("canonical validation payload is malformed") from exc
    if type(payload) is not dict or contract.canonical_json_bytes(payload) != payload_bytes:
        raise ReadbackError("validation payload bytes are not one canonical JSON object")
    return payload


def _recheck_comments(api: GitHubReadOnly, initial: Any, context: Any,
                      authority_record: Mapping[str, Any], run_created_at: str) -> Any:
    pages = api.issue_comment_pages()
    _, provisional = _resolve_records(pages)
    _compare_issue_authority(initial, provisional)
    contract.validate_authority_precedes_run(provisional, run_created_at)
    authority = contract.bind_recorded_admin_snapshot(provisional, authority_record)
    _compare_authority(initial, authority)
    bound = contract.bind_authority_context(authority, context)
    return bound


def _envelope(authority: Any, run: Mapping[str, Any], check: Mapping[str, Any],
              artifact: Mapping[str, Any], payload_bytes: bytes, archive_bytes: bytes) -> dict[str, Any]:
    record = contract.build_authority_record(authority, run)
    return {
        "schema": contract.READBACK_SCHEMA,
        "authority_digest": contract.authority_digest(record),
        "validation_id": authority.validation_id,
        "run_id": run["id"], "run_attempt": run["run_attempt"],
        "workflow_id": run["workflow_id"], "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"], "workflow_sha": run["workflow_sha"],
        "event": run["event"], "display_title": run["display_title"],
        "dispatched_head_sha": run["dispatched_head_sha"],
        "check_name": check["name"], "check_run_id": check["id"],
        "check_app_id": check["app_id"], "artifact_id": artifact["id"],
        "artifact_name": artifact["name"],
        "artifact_content_digest": contract.body_digest(archive_bytes),
        "payload_digest": contract.body_digest(payload_bytes),
    }


def read_validation(api: GitHubReadOnly | None = None) -> dict[str, Any]:
    """Perform one bounded readback pass; never dispatches, reruns, or writes."""
    api = api or GitHubReadOnly()
    issue = api.get_json(f"repos/{REPOSITORY}/issues/{TASK_ISSUE_NUMBER}")
    if not isinstance(issue, Mapping):
        raise ReadbackError("live Task Issue response is malformed")
    task_uid = _live_task_uid(issue)
    pr = api.get_json(f"repos/{REPOSITORY}/pulls/{PR_NUMBER}")
    if not isinstance(pr, Mapping):
        raise ReadbackError("live reciprocal PR response is malformed")
    pr_head, _ = _check_live_pr(pr, task_uid)
    initial_comments, provisional = _resolve_records(api.issue_comment_pages())
    request = provisional.request
    if (request["task_uid"] != task_uid or request["head_oid"] != pr_head
            or request["integration_base_oid"] != pr.get("base", {}).get("sha")):
        raise ReadbackError("frozen request differs from live Task UID or PR head")
    workflow_id, default_branch, repository_id = _live_workflow(api)
    pages = api.workflow_run_pages(workflow_id, repository_id)
    complete_runs = contract.collect_workflow_runs(pages)
    selected = contract.select_unique_run(complete_runs, provisional)
    run = _read_live_run(api, selected, workflow_id, default_branch)
    if run["display_title"] != contract.expected_run_title(provisional):
        raise ReadbackError("live selected run title differs from frozen request")
    contract.validate_authority_precedes_run(provisional, run["created_at"])
    if run["workflow_ref"] != WORKFLOW_REF or run["event"] != "workflow_dispatch":
        raise ReadbackError("selected run is not from the canonical default-branch dispatch")
    if run["workflow_sha"] != run["dispatched_head_sha"]:
        raise ReadbackError("run workflow SHA differs from dispatched workflow head")
    check = _latest_validation_check(api, run)
    artifact = _exact_artifact(api, run, provisional)
    archive_bytes = api.get_bytes(f"repos/{REPOSITORY}/actions/artifacts/{artifact['id']}/zip")
    payload_bytes = _payload_from_archive(archive_bytes)
    payload = _parse_payload_bytes(payload_bytes)
    authority_record = _authority_record_from_payload(payload)
    context_data = {
        "task_uid": task_uid,
        "head_oid": request["head_oid"],
        "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "repository_id": pr.get("base", {}).get("repo", {}).get("id"),
        "source_repository_id": pr.get("head", {}).get("repo", {}).get("id"),
    }
    # The exact W projection resolver and inventory replay verify H/S/D against
    # the live PR before an authorization context is built.
    trusted_context, merge_oid, tree_oid = _trusted_inventory(
        context_data, run, check, pr, initial_comments,
        run["workflow_sha"], default_branch,
    )
    authority = contract.bind_recorded_admin_snapshot(provisional, authority_record)
    authority = contract.bind_authority_context(authority, trusted_context)
    contract.validate_authority_precedes_run(authority, run["created_at"])
    run["tested_merge_oid"] = merge_oid
    run["tested_tree_oid"] = tree_oid
    contract.verify_payload(payload, authority, run, check)
    # Current collaborator permission is deliberately not reinterpreted as a
    # historical dispatch observation; exact W-issued admin snapshot is bound
    # only after run/check/artifact provenance and canonical payload bytes.
    final_authority = _recheck_comments(
        api, authority, trusted_context, authority_record, run["created_at"],
    )
    _compare_authority(authority, final_authority)
    final_pages = api.workflow_run_pages(workflow_id, repository_id)
    final_runs = contract.collect_workflow_runs(final_pages)
    final_selected = contract.select_unique_run(final_runs, final_authority)
    if final_selected.get("id") != run["id"] or final_selected.get("display_title") != run["display_title"]:
        raise ReadbackError("final full run-set enumeration differs from the initially selected R")
    final_run = _read_live_run(api, final_selected, workflow_id, default_branch)
    if (final_run["id"] != run["id"] or final_run["run_attempt"] != run["run_attempt"]
            or final_run["status"] != "completed" or final_run["conclusion"] != "success"
            or final_run["workflow_id"] != run["workflow_id"]
            or final_run["workflow_path"] != run["workflow_path"]
            or final_run["workflow_ref"] != run["workflow_ref"]
            or final_run["event"] != run["event"]
            or final_run["display_title"] != run["display_title"]
            or final_run["workflow_sha"] != run["workflow_sha"]
            or final_run["dispatched_head_sha"] != run["dispatched_head_sha"]):
        raise ReadbackError("final live run is not the same successful latest R/A")
    final_run["tested_merge_oid"] = merge_oid
    final_run["tested_tree_oid"] = tree_oid
    envelope = _envelope(final_authority, final_run, check, artifact, payload_bytes, archive_bytes)
    return contract.verify_readback(
        envelope, final_authority, final_run, check, artifact, payload_bytes, archive_bytes,
    )


def main() -> None:
    if len(sys.argv) != 1:
        raise SystemExit("ci-reuse-validation-readback accepts no arguments")
    try:
        value = read_validation()
    except (ReadbackError, contract.ContractError) as exc:
        raise SystemExit(f"ci-reuse-validation-readback: {exc}") from exc
    print(json.dumps(value, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
