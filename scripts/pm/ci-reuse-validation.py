#!/usr/bin/env python3
"""Issue and execute one isolated, trusted validation-only required-test run."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from typing import Any, Mapping
from urllib.parse import quote, urlsplit


HERE = Path(__file__).resolve().parent
_READBACK_SPEC = importlib.util.spec_from_file_location(
    "ci_reuse_validation_readback", HERE / "ci_reuse_validation_readback.py",
)
if _READBACK_SPEC is None or _READBACK_SPEC.loader is None:
    raise RuntimeError("validation readback adapter is unavailable")
readback = importlib.util.module_from_spec(_READBACK_SPEC)
sys.modules[_READBACK_SPEC.name] = readback
_READBACK_SPEC.loader.exec_module(readback)
contract = readback.contract


REPOSITORY = contract.REPOSITORY
TASK_ISSUE_NUMBER = contract.TASK_ISSUE_NUMBER
PR_NUMBER = contract.PR_NUMBER
WORKFLOW_FILE = contract.WORKFLOW_FILE
WORKFLOW_PATH = contract.WORKFLOW_PATH
WORKFLOW_REF = contract.WORKFLOW_REF
CHECK_NAME = contract.CHECK_NAME
GITHUB_ACTIONS_APP_ID = contract.GITHUB_ACTIONS_APP_ID
RUNNER_CONTRACT = "required-domain-split/v1"
RESULT_SCHEMA = "oasis7-ci-reuse-validation-unit-result/v1"
_OID_RE = re.compile(r"[0-9a-f]{40}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_INERT_INPUT_DEFAULTS = {
    "newapi_bridge_build_profile": "release",
    "escalation_reason": "",
    "evidence_url": "",
    "impact_projection_b64": "",
    "validation_request_b64": "",
}


class ProducerError(ValueError):
    """Trusted validation-only producer evidence was unavailable or mismatched."""


def validate_dispatch_inputs(actual: Any, expected: Mapping[str, str]) -> dict[str, str]:
    """Check all manual-dispatch assertions and reject any active alternate input."""
    if not isinstance(actual, Mapping) or not isinstance(expected, Mapping) or not expected:
        raise ProducerError("workflow_dispatch input objects are missing")
    for key, value in expected.items():
        if type(key) is not str or type(value) is not str:
            raise ProducerError("derived workflow_dispatch assertions are malformed")
    allowed = set(expected) | set(_INERT_INPUT_DEFAULTS)
    if not set(actual).issubset(allowed):
        raise ProducerError("workflow_dispatch contains an unknown assertion or selector")
    for key, value in expected.items():
        observed = actual.get(key)
        if type(observed) is not str or observed != value:
            raise ProducerError(f"workflow_dispatch assertion differs from frozen request: {key}")
    for key, default in _INERT_INPUT_DEFAULTS.items():
        observed = actual.get(key, default)
        if type(observed) is not str or observed != default:
            raise ProducerError(f"non-validation workflow input must remain inert: {key}")
    return dict(expected)


def build_runner_environment(
    validation_units: tuple[str, ...] | list[str],
    capability_runners: Mapping[str, Any],
    selector_env: Mapping[str, str],
) -> dict[str, str]:
    """Derive only authorized required-tier selectors from exact W inventory names."""
    if type(validation_units) not in {tuple, list} or not validation_units:
        raise ProducerError("authorized validation units are not a non-empty sequence")
    units = list(validation_units)
    if (any(type(item) is not str for item in units)
            or units != sorted(set(units), key=lambda item: item.encode("ascii"))):
        raise ProducerError("authorized validation units are not canonical and unique")
    if not isinstance(capability_runners, Mapping) or not isinstance(selector_env, Mapping):
        raise ProducerError("exact-W required runner inventory is unavailable")
    allowed = set(capability_runners)
    if not set(units).issubset(allowed):
        raise ProducerError("authorized unit has no exact-W executable required runner")
    if "required_gate_baseline" not in allowed:
        raise ProducerError("exact-W required runner inventory has no mandatory baseline")
    for unit, runners in capability_runners.items():
        if (type(unit) is not str or type(runners) not in {tuple, list} or not runners
                or any(type(runner) is not str or not runner for runner in runners)):
            raise ProducerError("exact-W required runner inventory is malformed")
    if not set(selector_env).issubset(allowed):
        raise ProducerError("exact-W selector maps an unknown validation unit")
    if set(selector_env) != allowed - {"required_gate_baseline"}:
        raise ProducerError("exact-W selector map does not cover every executable capability")
    if len(set(selector_env.values())) != len(selector_env):
        raise ProducerError("exact-W capability selectors are not one-to-one")

    environment = {
        name: "false" for name in selector_env.values()
    }
    if any(type(name) is not str or not name.startswith("OASIS7_CI_RUN_")
           for name in environment):
        raise ProducerError("exact-W required selector name is malformed")
    for unit in units:
        selector = selector_env.get(unit)
        if selector is not None:
            environment[selector] = "true"

    # Required-domain-split treats these selectors as paired obligations.
    environment["OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS"] = (
        "true" if "net" in units else "false"
    )
    environment["OASIS7_CI_RUN_VIEWER_WASM_CHECK"] = (
        "true" if "viewer_js_required" in units else "false"
    )
    environment["OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK"] = (
        "true" if "pixel_world_bridge" in units else "false"
    )
    environment.update({
        "OASIS7_CI_EXECUTION_CONTRACT": RUNNER_CONTRACT,
        "INTEGRATION_MODE": contract.RUN_MODE,
        "OASIS7_CI_RUN_RUST_BASELINE": "true",
        "OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE": "false",
        "OASIS7_CI_RUN_PROVIDER_LIVE_GATE": "false",
        # The trusted producer installs the complete runner resource closure.
        "OASIS7_CI_NEEDS_PYTHON": "true",
        "OASIS7_CI_NEEDS_MARKDOWN": "true",
        "OASIS7_CI_NEEDS_RUST_TOOLCHAIN": "true",
        "OASIS7_CI_NEEDS_NODE": "true",
        "OASIS7_CI_NEEDS_SYSTEM_DEPS": "true",
        "OASIS7_CI_NEEDS_TRUNK": "true",
        "OASIS7_CI_NEEDS_WASM_TARGET": "true",
    })
    return environment


def build_unit_result_digests(
    validation_units: tuple[str, ...] | list[str],
    obligations: Mapping[str, tuple[str, ...] | list[str]],
    successful_output_digest: str,
    *, run_id: int, run_attempt: int, tested_merge_oid: str, tested_tree_oid: str,
) -> dict[str, str]:
    """Bind each authorized obligation set to the exact successful required-tier output."""
    units = list(validation_units)
    if (not units or units != sorted(set(units), key=lambda item: item.encode("ascii"))
            or not isinstance(obligations, Mapping) or set(obligations) != set(units)):
        raise ProducerError("result obligations do not exactly cover authorized validation units")
    if type(successful_output_digest) is not str or not _DIGEST_RE.fullmatch(successful_output_digest):
        raise ProducerError("successful required-tier output digest is malformed")
    if type(run_id) is not int or run_id <= 0 or type(run_attempt) is not int or run_attempt <= 0:
        raise ProducerError("result digest run identity is malformed")
    if any(type(value) is not str or not _OID_RE.fullmatch(value)
           for value in (tested_merge_oid, tested_tree_oid)):
        raise ProducerError("result digest M/T identity is malformed")
    result: dict[str, str] = {}
    for unit in units:
        values = obligations[unit]
        if (type(values) not in {tuple, list} or not values
                or any(type(value) is not str or not value for value in values)):
            raise ProducerError(f"result obligations are malformed for {unit}")
        statement = {
            "schema": RESULT_SCHEMA,
            "unit_id": unit,
            "obligations": list(values),
            "status": "success",
            "required_output_digest": successful_output_digest,
            "run_id": run_id,
            "run_attempt": run_attempt,
            "tested_merge_oid": tested_merge_oid,
            "tested_tree_oid": tested_tree_oid,
        }
        preimage = b"oasis7-ci-reuse-validation-unit-result/v1\x00" + contract.canonical_json_bytes(statement)
        result[unit] = "sha256:" + hashlib.sha256(preimage).hexdigest()
    return result


def _json_without_duplicate_keys(raw: bytes, label: str) -> Any:
    def unique_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        output: dict[str, Any] = {}
        for key, value in pairs:
            if key in output:
                raise ProducerError(f"{label} has a duplicate JSON key")
            output[key] = value
        return output

    try:
        return json.loads(raw.decode("utf-8", "strict"), object_pairs_hook=unique_pairs)
    except ProducerError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ProducerError(f"{label} is not valid UTF-8 JSON") from exc


def _git(root: Path, *args: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), *args], stderr=subprocess.PIPE, text=True,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ProducerError("trusted W/B/H/M/T Git observation failed") from exc


def _load_w_module(root: Path, name: str):
    path = root / "scripts" / "pm" / f"{name}.py"
    if not path.is_file() or path.is_symlink():
        raise ProducerError(f"exact-W helper is unavailable: {name}")
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ProducerError(f"exact-W helper cannot be loaded: {name}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(module)
    except (OSError, ImportError, ValueError) as exc:
        raise ProducerError(f"exact-W helper failed to load: {name}") from exc
    finally:
        sys.path.pop(0)
    return module


def _flatten_issue_pages(pages: tuple[Any, ...]) -> tuple[Mapping[str, Any], ...]:
    if not pages:
        raise ProducerError("complete Task Issue comment listing is unavailable")
    comments: list[Mapping[str, Any]] = []
    for page in pages:
        if type(page) is not list or any(not isinstance(item, Mapping) for item in page):
            raise ProducerError("Task Issue comment pagination is malformed")
        comments.extend(page)
    return tuple(comments)


def _live_task_uid(issue: Mapping[str, Any]) -> str:
    if issue.get("number") != TASK_ISSUE_NUMBER or type(issue.get("body")) is not str:
        raise ProducerError("live Task Issue identity is unavailable")
    matches = re.findall(r"(?m)^task_uid: (task_[0-9a-f]{32})\s*$", issue["body"])
    if len(matches) != 1:
        raise ProducerError("live Task Issue does not have one canonical Task UID")
    return matches[0]


def _check_live_pr(pr: Mapping[str, Any], task_uid: str) -> tuple[str, str]:
    base = pr.get("base")
    head = pr.get("head")
    base_repo = base.get("repo") if isinstance(base, Mapping) else None
    head_repo = head.get("repo") if isinstance(head, Mapping) else None
    if (pr.get("number") != PR_NUMBER or pr.get("state") != "open" or pr.get("merged") is not False
            or not isinstance(base, Mapping) or not isinstance(head, Mapping)
            or not isinstance(base_repo, Mapping) or not isinstance(head_repo, Mapping)
            or base_repo.get("full_name") != REPOSITORY
            or head_repo.get("full_name") != REPOSITORY
            or base.get("ref") != "main"):
        raise ProducerError("live reciprocal PR is closed, cross-repository, or targets another branch")
    head_oid = head.get("sha")
    base_oid = base.get("sha")
    if (type(head_oid) is not str or not _OID_RE.fullmatch(head_oid)
            or type(base_oid) is not str or not _OID_RE.fullmatch(base_oid)):
        raise ProducerError("live reciprocal PR H/B identities are malformed")
    body = pr.get("body")
    if (type(body) is not str or body.count(f"Task: {task_uid}") != 1
            or body.count(f"Refs #{TASK_ISSUE_NUMBER}") != 1):
        raise ProducerError("live reciprocal PR does not bind the canonical Task and Issue")
    return head_oid, base_oid


def _live_admin_permissions(api: Any, authority: Any) -> dict[str, dict[str, str]]:
    actors = sorted({authority.authorized_actor, authority.pin_actor})
    result: dict[str, dict[str, str]] = {}
    for login in actors:
        endpoint = f"repos/{REPOSITORY}/collaborators/{quote(login, safe='')}/permission"
        observation = api.get_json(endpoint)
        user = observation.get("user") if isinstance(observation, Mapping) else None
        if (not isinstance(user, Mapping) or user.get("login") != login
                or observation.get("permission") != "admin"):
            raise ProducerError(f"live canonical-repository admin permission is not verified for {login}")
        result[login] = {"login": login, "permission": "admin"}
    return result


def _live_workflow(api: Any) -> tuple[int, str]:
    repository = api.get_json(f"repos/{REPOSITORY}")
    if (not isinstance(repository, Mapping) or repository.get("full_name") != REPOSITORY
            or repository.get("default_branch") != "main"):
        raise ProducerError("canonical repository default-branch identity is unavailable")
    workflow = api.get_json(f"repos/{REPOSITORY}/actions/workflows/rust.yml")
    if (not isinstance(workflow, Mapping) or workflow.get("path") != WORKFLOW_FILE
            or workflow.get("state") != "active" or type(workflow.get("id")) is not int
            or workflow["id"] <= 0):
        raise ProducerError("canonical active rust.yml workflow identity is unavailable")
    return workflow["id"], repository["default_branch"]


def _check_dispatch_run(api: Any, run_id: int, workflow_id: int, default_branch: str,
                        expected_title: str, environment: Mapping[str, str]) -> dict[str, Any]:
    row = api.get_json(f"repos/{REPOSITORY}/actions/runs/{run_id}")
    if (not isinstance(row, Mapping) or type(row.get("id")) is not int
            or row.get("id") != run_id):
        raise ProducerError("current workflow run direct identity is unavailable")
    head_sha = row.get("head_sha")
    attempt = row.get("run_attempt")
    repository = row.get("repository")
    head_repository = row.get("head_repository")
    if (type(attempt) is not int or attempt <= 0 or type(head_sha) is not str
            or not _OID_RE.fullmatch(head_sha)
            or type(row.get("workflow_id")) is not int or row.get("workflow_id") != workflow_id
            or row.get("path") != WORKFLOW_PATH
            or row.get("event") != "workflow_dispatch"
            or row.get("head_branch") != default_branch
            or row.get("display_title") != expected_title
            or not isinstance(repository, Mapping) or repository.get("full_name") != REPOSITORY
            or not isinstance(head_repository, Mapping)
            or head_repository.get("full_name") != REPOSITORY):
        raise ProducerError("current R is not the exact canonical default-branch validation dispatch")
    workflow_sha = environment.get("GITHUB_WORKFLOW_SHA")
    if (type(workflow_sha) is not str or not _OID_RE.fullmatch(workflow_sha)
            or head_sha != workflow_sha or environment.get("GITHUB_SHA") != workflow_sha
            or environment.get("GITHUB_REF") != f"refs/heads/{default_branch}"
            or environment.get("GITHUB_WORKFLOW_REF") != WORKFLOW_REF):
        raise ProducerError("actual workflow W differs from live default-branch run identity")
    return {
        "repository": REPOSITORY,
        "id": run_id,
        "workflow_id": workflow_id,
        "workflow_path": row["path"],
        "workflow_ref": WORKFLOW_REF,
        "workflow_sha": workflow_sha,
        "event": row["event"],
        "display_title": row["display_title"],
        "dispatched_head_sha": head_sha,
        "run_attempt": attempt,
        "created_at": row.get("created_at"),
        "status": row.get("status"),
        "conclusion": row.get("conclusion"),
    }


def _collection_rows(pages: tuple[Any, ...], key: str, label: str) -> tuple[Mapping[str, Any], ...]:
    if not pages:
        raise ProducerError(f"exact R/A {label} are unavailable")
    expected_total: int | None = None
    rows: list[Mapping[str, Any]] = []
    seen: set[int] = set()
    for index, page in enumerate(pages):
        if not isinstance(page, Mapping) or type(page.get("total_count")) is not int:
            raise ProducerError(f"{label} pagination total is malformed")
        total = page["total_count"]
        if total < 0 or (expected_total is not None and expected_total != total):
            raise ProducerError(f"{label} pagination total changed")
        expected_total = total
        batch = page.get(key)
        if type(batch) is not list:
            raise ProducerError(f"{label} page is malformed")
        for row in batch:
            if (not isinstance(row, Mapping) or type(row.get("id")) is not int
                    or row["id"] <= 0 or row["id"] in seen):
                raise ProducerError(f"{label} contains a malformed or duplicate ID")
            seen.add(row["id"])
            rows.append(row)
        has_next = page.get("has_next")
        if type(has_next) is not bool or has_next != (index < len(pages) - 1):
            raise ProducerError(f"{label} pagination is incomplete or uncertain")
    if expected_total != len(seen):
        raise ProducerError(f"{label} total differs from the complete unique result set")
    return tuple(rows)


def _latest_validation_check(api: Any, run: Mapping[str, Any]) -> dict[str, Any]:
    rows = _collection_rows(api.job_pages(run["id"], run["run_attempt"]), "jobs", "attempt jobs")
    candidates: list[Mapping[str, Any]] = []
    for job in rows:
        if (job.get("run_id") != run["id"] or job.get("run_attempt") != run["run_attempt"]
                or job.get("head_sha") != run["dispatched_head_sha"]
                or type(job.get("name")) is not str):
            raise ProducerError("attempt job does not bind exact R/A/dispatched-head identity")
        if job["name"] == CHECK_NAME:
            candidates.append(job)
    if len(candidates) != 1:
        raise ProducerError("distinct validation-only check job is missing or ambiguous")
    job = candidates[0]
    check_url = urlsplit(str(job.get("check_run_url") or ""))
    match = re.fullmatch(
        rf"/repos/{re.escape(REPOSITORY)}/check-runs/([1-9][0-9]*)", check_url.path,
    )
    if (check_url.scheme != "https" or check_url.netloc != "api.github.com"
            or check_url.query or check_url.fragment or match is None):
        raise ProducerError("validation job does not provide a canonical check-run URL")
    check_id = int(match.group(1))
    check = api.get_json(f"repos/{REPOSITORY}/check-runs/{check_id}")
    app = check.get("app") if isinstance(check, Mapping) else None
    if (not isinstance(check, Mapping) or type(app) is not dict
            or check.get("id") != check_id or check.get("name") != CHECK_NAME
            or app.get("id") != GITHUB_ACTIONS_APP_ID
            or check.get("head_sha") != run["dispatched_head_sha"]):
        raise ProducerError("live validation check-run identity differs from exact R/A job")
    return {
        "id": check_id,
        "name": CHECK_NAME,
        "app_id": GITHUB_ACTIONS_APP_ID,
        "head_sha": run["dispatched_head_sha"],
        "run_id": run["id"],
        "run_attempt": run["run_attempt"],
        "status": check.get("status"),
        "conclusion": check.get("conclusion"),
    }


def _resolve_projection_digest(
    root: Path, pr: Mapping[str, Any], task_uid: str, head_oid: str, source_scope_oid: str,
    issue_comments: tuple[Mapping[str, Any], ...],
) -> str:
    publication_marker = "<!-- oasis7-ci-publication/v1 -->"
    binding_marker = "<!-- oasis7-ci-publication-binding/v1 -->"
    publications = [item["body"] for item in issue_comments
                    if type(item.get("body")) is str and publication_marker in item["body"]]
    bindings = [item["body"] for item in issue_comments
                if type(item.get("body")) is str and binding_marker in item["body"]]
    if len(publications) != 1 or len(bindings) != 1:
        raise ProducerError("Task Issue must have one exact CI publication and reciprocal PR binding")
    try:
        projection_contract = _load_w_module(root, "projection_publication_contract")
        _load_w_module(root, "pr_projection_journal")
        publication_module = _load_w_module(root, "pr_projection_publication")
        resolver = _load_w_module(root, "pr_projection_resolver")
        publication = publication_module.parse_publication_comment(publications[0])
        binding = publication_module.parse_publication_binding_comment(bindings[0])
        result = resolver.resolve(
            pr.get("body"), task_uid=task_uid, source_head_oid=head_oid,
            scope_base_oid=source_scope_oid, publication=publication, binding=binding,
            repository=REPOSITORY, pr_number=PR_NUMBER,
            planner_config_sha256=publication["planner_config_sha256"],
            required_protocol="v2",
            live={
                "repository": REPOSITORY,
                "pr_number": PR_NUMBER,
                "repository_id": pr.get("base", {}).get("repo", {}).get("id"),
                "source_repository_id": pr.get("head", {}).get("repo", {}).get("id"),
                "head_oid": pr.get("head", {}).get("sha"),
                "state": pr.get("state"),
                "merged": pr.get("merged"),
            },
        )
        body_contract = projection_contract.decode_marker(pr.get("body"))
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise ProducerError("trusted exact-W v2 PR projection could not be resolved") from exc
    digest = result.get("projection_digest") if isinstance(result, Mapping) else None
    if (type(digest) is not str or not _DIGEST_RE.fullmatch(digest)
            or body_contract.get("task_uid") != task_uid
            or body_contract.get("source_head_oid") != head_oid
            or body_contract.get("scope_base_oid") != source_scope_oid
            or body_contract.get("projection_digest") != digest):
        raise ProducerError("trusted PR projection D differs from Task/H/S identity")
    return digest


def _prepare_inventory(
    root: Path, authority: Any, run: Mapping[str, Any], check: Mapping[str, Any],
    issue_comments: tuple[Mapping[str, Any], ...], pr: Mapping[str, Any], default_branch: str,
    temp_root: Path,
) -> tuple[Any, str, str, Path, Any]:
    request = authority.request
    task_uid = request["task_uid"]
    base_oid = request["integration_base_oid"]
    head_oid = request["head_oid"]
    scope_oid = request["source_scope_oid"]
    workflow_sha = run["workflow_sha"]
    if _git(root, "rev-parse", "--verify", "HEAD^{commit}") != workflow_sha:
        raise ProducerError("checked-out trusted W differs from actual GITHUB_WORKFLOW_SHA")
    if _git(root, "status", "--porcelain"):
        raise ProducerError("trusted W checkout is not clean before exact planner execution")
    try:
        subprocess.run([
            "git", "-C", str(root), "fetch", "--no-tags", "origin",
            f"+refs/pull/{PR_NUMBER}/head:refs/remotes/origin/pr-{PR_NUMBER}",
        ], check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ProducerError("exact live H cannot be fetched into trusted W") from exc
    if (_git(root, "rev-parse", "--verify", f"{base_oid}^{{commit}}") != base_oid
            or _git(root, "rev-parse", "--verify", f"{head_oid}^{{commit}}") != head_oid
            or _git(root, "rev-parse", "--verify", f"{workflow_sha}^{{commit}}") != workflow_sha):
        raise ProducerError("exact W/B/H commit object is unavailable")
    merge_bases = _git(root, "merge-base", "--all", base_oid, head_oid).splitlines()
    if merge_bases != [scope_oid]:
        raise ProducerError("live B/H does not have exactly the frozen source scope S")
    projection_digest = _resolve_projection_digest(
        root, pr, task_uid, head_oid, scope_oid, issue_comments,
    )
    if projection_digest != request["projection_digest"]:
        raise ProducerError("live v2 PR projection digest differs from frozen request")

    integration = _load_w_module(root, "integration_ci")
    m_root = temp_root / "target-m"
    try:
        composition = integration.compose(root, base_oid, head_oid, str(m_root))
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        raise ProducerError("exact W failed to compose frozen B/H tested target M") from exc
    merge_oid = composition.get("tested_commit_oid")
    tree_oid = composition.get("tested_tree_oid")
    if (type(merge_oid) is not str or not _OID_RE.fullmatch(merge_oid)
            or type(tree_oid) is not str or not _OID_RE.fullmatch(tree_oid)
            or _git(m_root, "rev-parse", "HEAD") != merge_oid
            or _git(m_root, "show", "-s", "--format=%T", "HEAD") != tree_oid):
        raise ProducerError("exact W composition did not produce verified M/T")
    changed_paths = _git(root, "diff", "--name-only", f"{scope_oid}..{head_oid}").splitlines()
    if changed_paths != sorted(set(changed_paths)):
        raise ProducerError("complete exact S..H changed paths are not canonical")

    planner_path = root / "scripts" / "plan-rust-required-scope.py"
    config_path = root / "scripts" / "ci-required-scope.v2.json"
    planner_command = [
        sys.executable, "-I", str(planner_path), "--event-name", "workflow_dispatch",
        "--run-mode", "legacy", "--config", str(config_path),
        "--base-ref", base_oid, "--head-ref", head_oid,
        "--task-uid", task_uid, "--scope-base-oid", scope_oid,
    ]
    for path in changed_paths:
        planner_command.extend(("--changed-path", path))
    try:
        planner_output = subprocess.run(
            planner_command, cwd=root, check=True, capture_output=True, text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ProducerError("exact-W complete required inventory planner replay failed") from exc
    inventory_module = _load_w_module(root, "ci_required_inventory")
    try:
        inventory = inventory_module.build_required_inventory(
            root, m_root, merge_oid, planner_output,
            repository=REPOSITORY, workflow_ref=WORKFLOW_REF,
            planner_authority_oid=workflow_sha, event_name="workflow_dispatch",
            run_mode="legacy", changed_paths=changed_paths,
            base_ref=base_oid, head_ref=head_oid, task_uid=task_uid,
            scope_base_oid=scope_oid, impact_projection=None,
            run_id=run["id"], run_attempt=run["run_attempt"],
            check_app_id=check["app_id"], check_run_id=check["id"],
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise ProducerError("exact-W complete required inventory could not be rebuilt") from exc
    specs = inventory.get("unit_specs") if isinstance(inventory, Mapping) else None
    inventory_ids = inventory.get("planner_inventory_issuer", {}).get("unit_ids") if isinstance(inventory, Mapping) else None
    if (type(specs) is not list or type(inventory_ids) is not list or not inventory_ids
            or any(type(item) is not str for item in inventory_ids)
            or inventory_ids != sorted(set(inventory_ids))):
        raise ProducerError("exact-W complete required inventory identity is malformed")
    obligations: dict[str, tuple[str, ...]] = {}
    for spec in specs:
        if not isinstance(spec, Mapping) or type(spec.get("unit_id")) is not str:
            raise ProducerError("exact-W required inventory has a malformed unit spec")
        unit = spec["unit_id"]
        values = spec.get("obligation_set")
        if unit in obligations or type(values) is not list or not values:
            raise ProducerError("exact-W required inventory unit obligations are missing or duplicated")
        if any(type(value) is not str or not value for value in values):
            raise ProducerError(f"exact-W required inventory obligations are malformed for {unit}")
        policy = spec.get("applicable_policy")
        execution_environment = spec.get("environment_contract")
        if (not isinstance(policy, Mapping) or policy.get("reuse_eligible") is not False
                or not isinstance(execution_environment, Mapping)
                or execution_environment.get("reuse_eligible") is not False):
            raise ProducerError(f"production reuse is not disabled for validation unit {unit}")
        obligations[unit] = tuple(values)
    if set(obligations) != set(inventory_ids):
        raise ProducerError("exact-W obligation sets do not cover the complete planner inventory")
    closure = inventory.get("input_scope", {}).get("closure_status", {})
    if (inventory.get("closure_status") not in {"complete", "unknown"}
            or not isinstance(closure, Mapping)
            or closure.get("status") != inventory.get("closure_status")
            or sorted(inventory.get("input_scope", {}).get("required_test_units", [])) != inventory_ids
            or inventory.get("product_corpus", {}).get("status") != "complete"):
        raise ProducerError("exact-W required inventory closure or full unit list is malformed")
    context = contract.TrustedRequestContext(
        task_uid=task_uid, head_oid=head_oid, source_scope_oid=scope_oid,
        projection_digest=projection_digest,
        planner_unit_ids=tuple(inventory_ids), planner_unit_obligations=obligations,
    )
    return context, merge_oid, tree_oid, m_root, inventory_module


def _stream_required_tier(
    root: Path, target_root: Path, authority: Any, merge_oid: str, tree_oid: str,
    environment: Mapping[str, str],
) -> str:
    command = [
        "bash", str(root / "scripts" / "ci-tests.sh"), "required",
        "--repo-root", str(target_root),
    ]
    process_environment = dict(os.environ)
    process_environment.pop("RUSTC_WRAPPER", None)
    process_environment.update(environment)
    request = authority.request
    process_environment.update({
        "CI_VERBOSE": "1",
        "INTEGRATION_WORKTREE": str(target_root),
        "OASIS7_PRODUCT_DOC_BASE": request["source_scope_oid"],
        "OASIS7_PRODUCT_DOC_HEAD": request["head_oid"],
        "OASIS7_CARGO_SCOPE_BASE": request["source_scope_oid"],
        "OASIS7_CARGO_SCOPE_HEAD": request["head_oid"],
        "OASIS7_CARGO_SCOPE_INTEGRATION_BASE": request["integration_base_oid"],
        "OASIS7_CARGO_STAGE_CHECK_HEAD": request["head_oid"],
        "OASIS7_CARGO_STAGE_PR_NUMBER": "",
        "OASIS7_CARGO_STAGE_TASK_UID": "",
        "OASIS7_CARGO_STAGE_RECEIPT": "",
    })
    digest = hashlib.sha256()
    try:
        process = subprocess.Popen(
            command, cwd=root, env=process_environment,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        )
    except OSError as exc:
        raise ProducerError("trusted required-tier runner could not start") from exc
    assert process.stdout is not None
    while True:
        chunk = process.stdout.read(64 * 1024)
        if not chunk:
            break
        digest.update(chunk)
        sys.stdout.buffer.write(chunk)
        sys.stdout.buffer.flush()
    return_code = process.wait()
    if return_code != 0:
        raise ProducerError(f"authorized required test tier failed with exit code {return_code}")
    return "sha256:" + digest.hexdigest()


def _install_target_node_dependencies(target_root: Path, validation_units: tuple[str, ...]) -> None:
    node_units = {"viewer_js_required", "viewer_performance_report", "launcher_web"}
    if not node_units.intersection(validation_units):
        return
    try:
        subprocess.run(
            ["npm", "ci", "--prefix", str(target_root / "crates" / "oasis7_viewer")],
            check=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ProducerError("exact M viewer dependencies could not be installed") from exc


def _payload(authority: Any, run: Mapping[str, Any], check: Mapping[str, Any],
             merge_oid: str, tree_oid: str, result_digests: Mapping[str, str]) -> dict[str, Any]:
    record = contract.build_authority_record(authority, run)
    payload = {
        **{key: value for key, value in record.items() if key != "schema"},
        "schema": contract.PAYLOAD_SCHEMA,
        "authority_digest": contract.authority_digest(record),
        "capability_under_test": contract.CAPABILITY,
        "tested_merge_oid": merge_oid,
        "tested_tree_oid": tree_oid,
        "run_attempt": run["run_attempt"],
        "event_inputs": contract.expected_event_inputs(authority),
        "check_name": check["name"],
        "check_run_id": check["id"],
        "check_app_id": check["app_id"],
        "selected_obligations": {
            unit: list(authority.planner_unit_obligations[unit])
            for unit in authority.request["validation_units"]
        },
        "result_digests": dict(result_digests),
    }
    contract.verify_payload(payload, authority, {**run, "tested_merge_oid": merge_oid,
                                                 "tested_tree_oid": tree_oid}, check)
    return payload


def produce_validation(
    *, api: Any | None = None, environment: Mapping[str, str] | None = None,
    event_path: str | Path | None = None, root: str | Path | None = None,
) -> dict[str, Any]:
    """Authenticate current R, replay exact W inventory, execute M, and write payload."""
    environment = dict(os.environ if environment is None else environment)
    api = api or readback.GitHubReadOnly()
    workspace = Path(root or environment.get("GITHUB_WORKSPACE", "")).resolve()
    event_file = Path(event_path or environment.get("GITHUB_EVENT_PATH", "")).resolve()
    if not workspace.is_dir() or not event_file.is_file():
        raise ProducerError("trusted workflow workspace or event input file is unavailable")
    if (environment.get("GITHUB_REPOSITORY") != REPOSITORY
            or environment.get("GITHUB_EVENT_NAME") != "workflow_dispatch"):
        raise ProducerError("validation-only producer requires the canonical manual dispatch")
    if (environment.get("GITHUB_WORKFLOW_REF") != WORKFLOW_REF
            or environment.get("GITHUB_REF") != "refs/heads/main"):
        raise ProducerError("validation-only producer is not executing on trusted main rust.yml")

    try:
        event = _json_without_duplicate_keys(event_file.read_bytes(), "GitHub workflow event")
    except OSError as exc:
        raise ProducerError("workflow_dispatch event bytes are unavailable") from exc
    if not isinstance(event, Mapping) or not isinstance(event.get("inputs"), Mapping):
        raise ProducerError("workflow_dispatch has no typed inputs object")

    task_issue = api.get_json(f"repos/{REPOSITORY}/issues/{TASK_ISSUE_NUMBER}")
    if not isinstance(task_issue, Mapping):
        raise ProducerError("canonical Task Issue live read is malformed")
    task_uid = _live_task_uid(task_issue)
    pr = api.get_json(f"repos/{REPOSITORY}/pulls/{PR_NUMBER}")
    if not isinstance(pr, Mapping):
        raise ProducerError("reciprocal PR live read is malformed")
    head_oid, base_oid = _check_live_pr(pr, task_uid)

    comments = _flatten_issue_pages(api.issue_comment_pages())
    provisional = contract.resolve_records_for_readback(comments)
    if (provisional.request["task_uid"] != task_uid
            or provisional.request["head_oid"] != head_oid
            or provisional.request["integration_base_oid"] != base_oid):
        raise ProducerError("frozen request differs from live Task UID or PR H/B")
    permissions = _live_admin_permissions(api, provisional)
    authority = contract.resolve_records(comments, permissions)
    workflow_id, default_branch = _live_workflow(api)

    expected_inputs = contract.expected_event_inputs(authority)
    validate_dispatch_inputs(event["inputs"], expected_inputs)
    expected_title = contract.expected_run_title(authority)
    workflow_pages = api.workflow_run_pages(workflow_id)
    complete_runs = contract.collect_workflow_runs(workflow_pages)
    try:
        selected = contract.select_unique_run(complete_runs, authority)
    except contract.ContractError as exc:
        raise ProducerError("request does not resolve to exactly one canonical W run") from exc

    run_id = environment.get("GITHUB_RUN_ID")
    attempt = environment.get("GITHUB_RUN_ATTEMPT")
    if (type(run_id) is not str or not run_id.isdigit() or int(run_id) <= 0
            or type(attempt) is not str or not attempt.isdigit() or int(attempt) <= 0
            or selected.get("id") != int(run_id)):
        raise ProducerError("current R/A is not the sole uniquely selected validation dispatch")
    run = _check_dispatch_run(api, int(run_id), workflow_id, default_branch, expected_title, environment)
    if run["run_attempt"] != int(attempt):
        raise ProducerError("current live run attempt differs from GITHUB_RUN_ATTEMPT")
    contract.validate_authority_precedes_run(authority, run["created_at"])

    check = _latest_validation_check(api, run)
    with tempfile.TemporaryDirectory(prefix="oasis7-ci-reuse-validation-") as temp_name:
        temp_root = Path(temp_name)
        context, merge_oid, tree_oid, target_root, inventory_module = _prepare_inventory(
            workspace, authority, run, check, comments, pr, default_branch, temp_root,
        )
        authority = contract.bind_authority_context(authority, context)
        units = authority.request["validation_units"]
        selector_environment = build_runner_environment(
            units, inventory_module.CAPABILITY_RUNNERS, inventory_module.SELECTOR_ENV,
        )
        _install_target_node_dependencies(target_root, units)
        output_digest = _stream_required_tier(
            workspace, target_root, authority, merge_oid, tree_oid, selector_environment,
        )
        result_digests = build_unit_result_digests(
            units, authority.planner_unit_obligations, output_digest,
            run_id=run["id"], run_attempt=run["run_attempt"],
            tested_merge_oid=merge_oid, tested_tree_oid=tree_oid,
        )
        payload = _payload(authority, run, check, merge_oid, tree_oid, result_digests)

        # The full request-to-run barrier is repeated after tests. A second R or
        # an attempt advance cannot inherit this attempt's successful output.
        final_pages = api.workflow_run_pages(workflow_id)
        final_runs = contract.collect_workflow_runs(final_pages)
        final_selected = contract.select_unique_run(final_runs, authority)
        if final_selected.get("id") != run["id"] or final_selected.get("display_title") != expected_title:
            raise ProducerError("final full run history differs from the unique pre-test R")
        final_run = _check_dispatch_run(
            api, run["id"], workflow_id, default_branch, expected_title, environment,
        )
        if final_run["run_attempt"] != run["run_attempt"]:
            raise ProducerError("same R advanced to a newer attempt during authorized execution")

        output_path = workspace / "output" / "ci-reuse-validation" / contract.PAYLOAD_MEMBER
        if output_path.exists() or output_path.is_symlink():
            raise ProducerError("validation payload output path already exists")
        output_path.parent.mkdir(parents=True, exist_ok=True)
        raw = contract.canonical_json_bytes(payload)
        with output_path.open("xb") as stream:
            stream.write(raw)
    return payload


def main() -> None:
    if len(sys.argv) != 1:
        raise SystemExit("ci-reuse-validation accepts no arguments")
    try:
        payload = produce_validation()
    except (ProducerError, contract.ContractError, readback.ReadbackError) as exc:
        raise SystemExit(f"ci-reuse-validation: {exc}") from exc
    print(json.dumps({
        "schema": payload["schema"],
        "validation_id": payload["validation_id"],
        "run_id": payload["run_id"],
        "run_attempt": payload["run_attempt"],
        "tested_merge_oid": payload["tested_merge_oid"],
        "tested_tree_oid": payload["tested_tree_oid"],
    }, sort_keys=True, separators=(",", ":")))
    output_path = os.environ.get("GITHUB_OUTPUT")
    if output_path:
        artifact = contract.artifact_name(
            payload["validation_id"], payload["run_id"], payload["run_attempt"],
        )
        with Path(output_path).open("a", encoding="utf-8") as stream:
            stream.write("artifact_name=" + artifact + "\n")


if __name__ == "__main__":
    main()
