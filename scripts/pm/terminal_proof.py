"""Canonical identity for repository-produced merged-terminal receipts."""
from __future__ import annotations

import datetime as dt
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import urllib.parse


def receipt_chain_digest(
    task_uid: str,
    repository: str,
    issue_number: int,
    pr_number: int,
    pr_url: str,
    merge_receipt_sha256: str,
    main_sync_receipt_sha256: str,
    terminal_receipt_sha256: str,
) -> str:
    """Bind task/PR identity and all producer receipt digests to one value."""
    payload = {
        "task_uid": task_uid,
        "repository": repository,
        "issue_number": issue_number,
        "pr_number": pr_number,
        "pr_url": pr_url,
        "merge_receipt_sha256": merge_receipt_sha256,
        "main_sync_receipt_sha256": main_sync_receipt_sha256,
        "terminal_receipt_sha256": terminal_receipt_sha256,
    }
    canonical = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _receipt(root: pathlib.Path, name: str) -> dict:
    path = root / name
    if not path.is_file():
        raise ValueError(f"canonical terminal receipt missing: {name}")
    raw = path.read_bytes()
    try:
        record = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"canonical terminal receipt is not valid JSON: {name}") from exc
    if not isinstance(record, dict):
        raise ValueError(f"canonical terminal receipt is not an object: {name}")
    return {"bytes": raw, "digest": hashlib.sha256(raw).hexdigest(), "record": record}


def read_receipt_chain(repo_root: pathlib.Path, task_uid: str) -> dict:
    """Read only the task UID's repository-owned canonical receipt root."""
    helper = pathlib.Path(__file__).with_name("canonical-receipt-root.py")
    try:
        raw = subprocess.check_output(
            [sys.executable, str(helper), "--default-worktree", str(repo_root),
             "--task-uid", task_uid, "--json"],
            text=True,
            stderr=subprocess.PIPE,
        )
        root = pathlib.Path(json.loads(raw)["receipt_root"])
    except (OSError, subprocess.SubprocessError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise ValueError("canonical terminal receipt root unavailable") from exc
    receipts = {
        "merge": _receipt(root, "merge-receipt.json"),
        "main_sync": _receipt(root, "main-sync-receipt.json"),
        "terminal": _receipt(root, "terminal-cleanup-receipt.json"),
        "ledger": _receipt(root, "finalizer-ledger.json"),
        "tombstone": _receipt(root, "terminal-tombstone.json"),
    }
    if receipts["main_sync"]["record"].get("integration_mode") == "patch_equivalence":
        receipts["patch_equivalence"] = _receipt(root, "patch-equivalence-receipt.json")
    return receipts


def validate_receipt_chain(
    receipts: dict,
    *,
    repository: str,
    task_uid: str,
    issue_number: int,
    pr_number: int,
    pr_url: str,
    pr: dict,
) -> dict[str, str]:
    """Validate producer schemas and return digests of their actual bytes."""
    required = ("merge", "main_sync", "terminal", "ledger", "tombstone")
    if any(not isinstance(receipts.get(name), dict) for name in required):
        raise ValueError("canonical terminal receipt chain is incomplete")
    merge = receipts["merge"]["record"]
    main_sync = receipts["main_sync"]["record"]
    terminal = receipts["terminal"]["record"]
    ledger = receipts["ledger"]["record"]
    tombstone = receipts["tombstone"]["record"]
    merge_digest = receipts["merge"]["digest"]
    main_sync_digest = receipts["main_sync"]["digest"]
    terminal_digest = receipts["terminal"]["digest"]
    for key, expected in {
        "receipt_type": "oasis7_pr_merge",
        "issuer": "github_live_query",
        "evidence_mode": "production",
        "repository": repository,
        "pr_number": pr_number,
        "pr_url": pr_url,
        "state": "MERGED",
    }.items():
        if merge.get(key) != expected:
            raise ValueError("canonical merge receipt provenance mismatch")
    if (not isinstance(merge.get("default_branch"), str) or not merge.get("default_branch")
            or not isinstance(merge.get("base_ref"), str) or not merge.get("base_ref")):
        raise ValueError("canonical merge receipt branch provenance missing")
    if (not merge.get("merged_at") or not merge.get("observed_at")
            or not re_fullmatch_oid(merge.get("head_oid"))):
        raise ValueError("canonical merge receipt lacks merged version")
    if merge.get("base_ref") != merge.get("default_branch"):
        raise ValueError("canonical merge receipt targets a non-default branch")
    live_base = ((pr.get("base") or {}).get("ref"))
    if live_base and live_base != merge.get("base_ref"):
        raise ValueError("canonical merge receipt base branch disagrees with live PR")
    live_head = ((pr.get("head") or {}).get("sha"))
    if live_head and live_head != merge.get("head_oid"):
        raise ValueError("canonical merge receipt head version disagrees with live PR")
    if (main_sync.get("receipt_type"), main_sync.get("issuer"), main_sync.get("task_uid"),
            main_sync.get("repository"), main_sync.get("default_branch"),
            main_sync.get("merge_receipt_sha256")) != (
            "oasis7_main_sync", "post-merge-main-sync", task_uid, repository,
            merge.get("default_branch"), merge_digest):
        raise ValueError("canonical main-sync receipt provenance mismatch")
    integration_mode = main_sync.get("integration_mode")
    if integration_mode not in ("ancestry", "patch_equivalence") or not main_sync.get("observed_at"):
        raise ValueError("canonical main-sync receipt integration mode is invalid")
    if not re_fullmatch_oid(main_sync.get("main_commit")) or not re_fullmatch_oid(main_sync.get("remote_main_commit")):
        raise ValueError("canonical main-sync receipt lacks synchronized producer commits")
    if main_sync.get("main_commit") != main_sync.get("remote_main_commit"):
        raise ValueError("canonical main-sync receipt local/remote commits disagree")
    patch_fields = (
        "patch_equivalence_receipt_sha256", "patch_id", "projected_tree_oid",
        "main_tree_oid", "integration_commit", "integration_parent",
    )
    if integration_mode == "ancestry":
        if any(field in main_sync for field in patch_fields):
            raise ValueError("canonical ancestry main-sync receipt contains patch-equivalence fields")
    else:
        if not re_fullmatch_sha256(main_sync.get("patch_equivalence_receipt_sha256")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks patch receipt digest")
        if not re_fullmatch_oid(main_sync.get("patch_id")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks patch identity")
        if not re_fullmatch_oid(main_sync.get("projected_tree_oid")) or not re_fullmatch_oid(main_sync.get("main_tree_oid")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks tree identity")
        if not re_fullmatch_oid(main_sync.get("integration_commit")) or not re_fullmatch_oid(main_sync.get("integration_parent")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks integration commit identity")
        if main_sync.get("projected_tree_oid") != main_sync.get("main_tree_oid"):
            raise ValueError("canonical patch-equivalence main-sync trees disagree")
        patch = receipts.get("patch_equivalence")
        if not isinstance(patch, dict) or not isinstance(patch.get("record"), dict):
            raise ValueError("canonical patch-equivalence receipt is unavailable")
        patch_record = patch["record"]
        if patch.get("digest") != main_sync.get("patch_equivalence_receipt_sha256"):
            raise ValueError("canonical patch-equivalence receipt digest disagrees with main-sync")
        expected_patch = {
            "receipt_type": "oasis7_patch_equivalence",
            "schema_version": 2,
            "issuer": "oasis7_patch_equivalence_helper",
            "branch_tip": merge.get("head_oid"),
            "main_commit": main_sync.get("integration_commit"),
            "main_parent": main_sync.get("integration_parent"),
            "patch_id": main_sync.get("patch_id"),
            "projected_tree_oid": main_sync.get("projected_tree_oid"),
            "main_tree_oid": main_sync.get("main_tree_oid"),
        }
        if any(patch_record.get(key) != value for key, value in expected_patch.items()):
            raise ValueError("canonical patch-equivalence receipt provenance mismatch")
    if (terminal.get("receipt_type"), terminal.get("issuer"), terminal.get("task_uid"),
            terminal.get("repository"), terminal.get("issue_number"), terminal.get("pr_number"),
            terminal.get("merge_receipt_sha256"), terminal.get("main_sync_receipt_sha256")) != (
            "oasis7_terminal_cleanup", "post-merge-cleanup", task_uid, repository,
            issue_number, pr_number, merge_digest, main_sync_digest):
        raise ValueError("canonical terminal receipt provenance mismatch")
    if not terminal.get("worktree") or not terminal.get("branch") or not terminal.get("observed_at"):
        raise ValueError("canonical terminal receipt lacks worktree/branch provenance")
    if ledger.get("schema") != "oasis7_finalizer_ledger_v1" or ledger.get("task_uid") != task_uid:
        raise ValueError("canonical finalizer ledger provenance mismatch")
    operations = ledger.get("operations") or {}
    for effect in ("project_update", "evidence_comment", "issue_close"):
        entry = operations.get(effect) or {}
        operation_id = hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
        if (entry.get("effect"), entry.get("operation_id"), entry.get("committed")) != (effect, operation_id, True):
            raise ValueError(f"canonical finalizer ledger lacks committed {effect}")
    if (tombstone.get("schema"), tombstone.get("task_uid"), tombstone.get("repository"),
            tombstone.get("issue_number"), tombstone.get("pr_number"),
            tombstone.get("workflow_phase"), tombstone.get("terminal_receipt_sha256"),
            tombstone.get("checkout_recreation_forbidden")) != (
            "oasis7_terminal_tombstone_v1", task_uid, repository, issue_number,
            pr_number, "post_merge_done", terminal_digest, True):
        raise ValueError("canonical terminal tombstone provenance mismatch")
    if (tombstone.get("canonical_worktree"), tombstone.get("task_branch")) != (
            terminal.get("worktree"), terminal.get("branch")):
        raise ValueError("canonical terminal tombstone checkout provenance mismatch")
    return {
        "merge": merge_digest,
        "main_sync": main_sync_digest,
        "terminal": terminal_digest,
    }


def re_fullmatch_oid(value: object) -> bool:
    return isinstance(value, str) and len(value) == 40 and all(c in "0123456789abcdef" for c in value)


def re_fullmatch_sha256(value: object) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


# The v1 functions above are an immutable compatibility surface. V2 callers
# use this shared version-dispatching reader and never reinterpret v1 bytes.
DELIVERY_RECEIPT_FIELDS = frozenset({
    "receipt_type", "schema_version", "issuer", "evidence_mode", "task_uid",
    "repository", "issue_number", "pr_number", "pr_url", "head_oid",
    "merge_commit_oid", "default_branch", "observed_target_oid",
    "merge_receipt_sha256", "task_complete_claim_sha256", "worktree",
    "branch", "completion_semantics", "observed_at", "readiness_proof_sha256",
})
DELIVERY_MARKER = "<!-- oasis7-pm-evidence/v2 -->"
OID_RE = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")


def _canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _strict_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate JSON key")
        value[key] = item
    return value


def _load_json_bytes(raw: bytes, name: str) -> dict:
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_strict_object)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise ValueError(f"{name} is not unique-key UTF-8 JSON") from exc
    if not isinstance(value, dict):
        raise ValueError(f"{name} is not an object")
    return value


def _canonical_root(repo_root: pathlib.Path, task_uid: str) -> pathlib.Path:
    helper = pathlib.Path(__file__).with_name("canonical-receipt-root.py")
    try:
        output = subprocess.check_output(
            [sys.executable, str(helper), "--default-worktree", str(repo_root),
             "--task-uid", task_uid, "--json"], text=True, stderr=subprocess.PIPE,
        )
        return pathlib.Path(json.loads(output)["receipt_root"])
    except (OSError, subprocess.SubprocessError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise ValueError("canonical terminal receipt root unavailable") from exc


def _read_v2_files(repo_root: pathlib.Path, task_uid: str) -> dict[str, dict]:
    root = _canonical_root(repo_root, task_uid)
    result = {"root": repo_root}
    for key, filename in (
        ("merge", "merge-receipt.json"),
        ("delivery", "terminal-delivery-receipt.json"),
        ("ledger", "finalizer-ledger.json"),
        ("tombstone", "terminal-tombstone.json"),
    ):
        path = root / filename
        try:
            raw = path.read_bytes()
        except OSError as exc:
            label = "terminal delivery receipt" if key == "delivery" else filename
            raise ValueError(f"{label} is unavailable") from exc
        result[key] = {"bytes": raw, "digest": hashlib.sha256(raw).hexdigest(),
                       "record": _load_json_bytes(raw, filename)}
    return result


def _one_markdown(body: object, key: str) -> str | None:
    matches = re.findall(rf"^- {re.escape(key)}: `([^`]+)`$", str(body or "").replace("\r\n", "\n"), re.MULTILINE)
    return matches[0] if len(matches) == 1 else None


def _has_task_uid(body: object, task_uid: str) -> bool:
    matches = re.findall(r"^task_uid:\s*([^\n]+)$", str(body or "").replace("\r\n", "\n"), re.MULTILINE)
    return matches == [task_uid]


def _validate_mapping_identity(task_uid: str, task_record: dict, repository: str,
                               issue_number: int, pr_number: int, pr_url: str) -> None:
    if task_record.get("task_uid") not in (None, task_uid):
        raise ValueError("terminal delivery task mapping UID mismatch")
    for key, value in {"repository": repository, "issue_number": issue_number,
                       "pr_number": pr_number, "pr_url": pr_url}.items():
        if task_record.get(key) != value:
            raise ValueError(f"terminal delivery task mapping {key} mismatch")


def _validate_live_issue(live_issue: dict, repository: str, task_uid: str,
                         issue_number: int, pr_number: int, pr_url: str) -> None:
    issue_url = f"https://github.com/{repository}/issues/{issue_number}"
    if (live_issue.get("number") != issue_number
            or live_issue.get("html_url", live_issue.get("url")) != issue_url
            or not _has_task_uid(live_issue.get("body"), task_uid)):
        raise ValueError("terminal delivery live Issue identity mismatch")
    if (_one_markdown(live_issue.get("body"), "pr_number") != str(pr_number)
            or _one_markdown(live_issue.get("body"), "pr_url") != pr_url):
        raise ValueError("terminal delivery live Issue PR binding mismatch")
    if (str(live_issue.get("state") or "").upper() != "CLOSED"
            or str(live_issue.get("state_reason", live_issue.get("stateReason", ""))).lower() != "completed"):
        raise ValueError("terminal delivery Issue is not closed as completed")


def _validate_live_pr(live_pr: dict, repository: str, task_uid: str,
                      issue_number: int, pr_number: int, pr_url: str,
                      head_oid: str, merge_commit_oid: str, default_branch: str) -> None:
    body = str(live_pr.get("body") or "").replace("\r\n", "\n")
    base = live_pr.get("base") or {}
    head = live_pr.get("head") or {}
    if (live_pr.get("number") != pr_number or live_pr.get("html_url") != pr_url
            or ((base.get("repo") or {}).get("full_name")) != repository
            or ((head.get("repo") or {}).get("full_name")) != repository):
        raise ValueError("terminal delivery live PR reciprocal identity mismatch")
    if (re.findall(r"^Task: [^\n]+$", body, re.MULTILINE) != [f"Task: {task_uid}"]
            or re.findall(r"^Refs #[1-9][0-9]*$", body, re.MULTILINE) != [f"Refs #{issue_number}"]):
        raise ValueError("terminal delivery live PR task reference mismatch")
    if (str(live_pr.get("state") or "").upper() != "CLOSED"
            or live_pr.get("merged") is not True or not live_pr.get("merged_at")
            or live_pr.get("merge_commit_sha") != merge_commit_oid
            or head.get("sha") != head_oid or base.get("ref") != default_branch):
        raise ValueError("terminal delivery live PR merge identity mismatch")


def read_live_repository(repository: str, merge_commit_oid: str,
                         observed_target_oid: str | None = None) -> dict:
    """Read exact live repository, default ref, and GitHub compare responses."""
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", repository) or not OID_RE.fullmatch(merge_commit_oid):
        raise ValueError("terminal delivery repository query identity is invalid")

    def query(endpoint: str) -> dict:
        try:
            raw = subprocess.check_output(["gh", "api", endpoint], text=True,
                                          stderr=subprocess.PIPE, timeout=180)
            value = json.loads(raw)
        except (OSError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
            raise ValueError("terminal delivery live repository readback failed") from exc
        if not isinstance(value, dict):
            raise ValueError("terminal delivery live repository readback is malformed")
        return value

    repo_response = query(f"repos/{repository}")
    branch = repo_response.get("default_branch")
    if not isinstance(branch, str) or not branch:
        raise ValueError("terminal delivery live default branch is unavailable")
    ref_response = query(f"repos/{repository}/git/ref/heads/{urllib.parse.quote(branch, safe='')}")
    target = ((ref_response.get("object") or {}).get("sha"))
    if not isinstance(target, str) or not OID_RE.fullmatch(target):
        raise ValueError("terminal delivery live default branch OID is invalid")
    compare = query(f"repos/{repository}/compare/{merge_commit_oid}...{target}")
    observed_compare = None
    if observed_target_oid is not None and observed_target_oid != target:
        if not OID_RE.fullmatch(observed_target_oid):
            raise ValueError("terminal delivery observed target OID is invalid")
        observed_compare = query(f"repos/{repository}/compare/{observed_target_oid}...{target}")
    return {"repository": repo_response, "ref": ref_response,
            "merge_compare": compare, "observed_target_compare": observed_compare}


def _compare_contains_base(compare: object, repository: str, base: str, target: str) -> bool:
    """Bind the real REST compare summary to independently resolved OIDs."""
    if not isinstance(compare, dict):
        return False
    if compare.get("url") != f"https://api.github.com/repos/{repository}/compare/{base}...{target}":
        return False
    for field in ("base_commit", "merge_base_commit"):
        commit = compare.get(field)
        if not isinstance(commit, dict) or commit.get("sha") != base:
            return False
    counts = [compare.get(field) for field in ("ahead_by", "behind_by", "total_commits")]
    if any(type(count) is not int or count < 0 for count in counts):
        return False
    ahead, behind, total = counts
    if behind != 0 or total != ahead:
        return False
    # Commit arrays can be truncated; endpoint identity and merge base establish
    # ancestry without guessing a head_commit field or inspecting the last item.
    return ((base == target and compare.get("status") == "identical" and ahead == 0)
            or (base != target and compare.get("status") == "ahead" and ahead > 0))


def _validate_live_repository(live_repository: dict, repository: str,
                              merge_commit_oid: str, default_branch: str,
                              observed_target_oid: str) -> str:
    repo = live_repository.get("repository") or {}
    ref = live_repository.get("ref") or {}
    compare = live_repository.get("merge_compare") or {}
    if (repo.get("full_name") != repository or repo.get("default_branch") != default_branch
            or ref.get("ref") != f"refs/heads/{default_branch}"):
        raise ValueError("terminal delivery live repository/default branch identity mismatch")
    target = ((ref.get("object") or {}).get("sha"))
    if not isinstance(target, str) or not OID_RE.fullmatch(target):
        raise ValueError("terminal delivery live target OID is invalid")
    if not _compare_contains_base(compare, repository, merge_commit_oid, target):
        raise ValueError("terminal delivery live merge is not contained in target history")
    if observed_target_oid != target:
        observed_compare = live_repository.get("observed_target_compare") or {}
        if not _compare_contains_base(observed_compare, repository, observed_target_oid, target):
            raise ValueError("terminal delivery observed target is not on live target history")
    return target


def validate_live_repository(live_repository: dict, repository: str,
                             merge_commit_oid: str, *,
                             default_branch: str | None = None,
                             observed_target_oid: str | None = None) -> str:
    """Validate an actual response bundle and return its current ref OID."""
    repo = live_repository.get("repository") or {}
    ref = live_repository.get("ref") or {}
    branch = default_branch or repo.get("default_branch")
    target = ((ref.get("object") or {}).get("sha"))
    if not isinstance(branch, str) or not isinstance(target, str):
        raise ValueError("terminal delivery live repository readback is incomplete")
    return _validate_live_repository(live_repository, repository, merge_commit_oid,
                                     branch, observed_target_oid or target)


def _project_values(item: dict) -> dict[str, object]:
    if not isinstance(item, dict) or not item.get("id"):
        raise ValueError("terminal delivery live Project item is unavailable")
    project = item.get("project") or {}
    values = item.get("fieldValues") or {}
    if (values.get("pageInfo") or {}).get("hasNextPage") is not False:
        raise ValueError("terminal delivery live Project field pagination is incomplete")
    fields: dict[str, str] = {}
    for value in values.get("nodes") or []:
        name = ((value.get("field") or {}).get("name"))
        if not isinstance(name, str) or name in fields:
            raise ValueError("terminal delivery live Project field is missing or duplicated")
        fields[name] = str(value.get("name", value.get("text", "")) or "")
    return {"id": str(item["id"]), "project_id": str(project.get("id") or ""),
            "project_number": project.get("number"),
            "project_owner": ((project.get("owner") or {}).get("login")),
            "content": item.get("content") or {}, "fields": fields}


def _v2_comment_payload(receipt: dict, delivery_digest: str) -> dict:
    payload = {
        "schema": "oasis7.terminal-delivery-comment/v2",
        "receipt_type": "oasis7_terminal_delivery", "receipt_chain_version": 2,
        "evidence_phase": "post_merge_done", "task_uid": receipt["task_uid"],
        "repository": receipt["repository"], "issue_number": receipt["issue_number"],
        "pr_number": receipt["pr_number"], "pr_url": receipt["pr_url"],
        "head_oid": receipt["head_oid"], "merge_commit_oid": receipt["merge_commit_oid"],
        "merge_receipt_sha256": receipt["merge_receipt_sha256"],
        "task_complete_claim_sha256": receipt["task_complete_claim_sha256"],
        "readiness_proof_sha256": receipt["readiness_proof_sha256"],
        "terminal_delivery_receipt_sha256": delivery_digest,
    }
    payload["receipt_chain_sha256"] = hashlib.sha256(_canonical_json(payload)).hexdigest()
    return payload


def _v2_comment_body(receipt: dict, delivery_digest: str) -> str:
    return DELIVERY_MARKER + "\n" + _canonical_json(_v2_comment_payload(receipt, delivery_digest)).decode("utf-8")


def terminal_delivery_comment_body(receipt: dict, delivery_digest: str) -> str:
    """Build the canonical v2 Issue evidence body from the receipt bytes digest."""
    return _v2_comment_body(receipt, delivery_digest)


def _validate_delivery_record(receipt: dict, files: dict[str, dict], task_uid: str,
                              task_record: dict, live_issue: dict, live_project_item: dict,
                              live_pr: dict, live_repository: dict, comments: list[dict]) -> dict:
    if set(receipt) != DELIVERY_RECEIPT_FIELDS:
        raise ValueError("terminal delivery receipt closed schema mismatch")
    if (receipt.get("receipt_type"), receipt.get("schema_version"), receipt.get("issuer"),
            receipt.get("evidence_mode"), receipt.get("completion_semantics")) != (
            "oasis7_terminal_delivery", 2, "post-merge-finalize", "production", "delivery_only"):
        raise ValueError("terminal delivery receipt provenance mismatch")
    repository = receipt.get("repository")
    issue_number = receipt.get("issue_number")
    pr_number = receipt.get("pr_number")
    pr_url = receipt.get("pr_url")
    if (receipt.get("task_uid") != task_uid or not isinstance(repository, str)
            or type(issue_number) is not int or issue_number <= 0
            or type(pr_number) is not int or pr_number <= 0
            or pr_url != f"https://github.com/{repository}/pull/{pr_number}"):
        raise ValueError("terminal delivery receipt identity mismatch")
    _validate_mapping_identity(task_uid, task_record, repository, issue_number, pr_number, pr_url)
    from readiness_transport import validate_readiness_proof
    readiness = validate_readiness_proof(
        pathlib.Path(files["root"]), task_uid, task_record,
        live_pr=live_pr, comments=comments, live_issue=live_issue)
    if receipt.get("readiness_proof_sha256") != readiness["digest"]:
        raise ValueError("terminal delivery readiness proof raw digest mismatch")
    for key in ("head_oid", "merge_commit_oid", "observed_target_oid"):
        if not isinstance(receipt.get(key), str) or not OID_RE.fullmatch(receipt[key]):
            raise ValueError(f"terminal delivery receipt {key} is invalid")
    if not re_fullmatch_sha256(receipt.get("merge_receipt_sha256")):
        raise ValueError("terminal delivery receipt merge digest is invalid")
    claim_hash = receipt.get("task_complete_claim_sha256")
    if not isinstance(claim_hash, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", claim_hash):
        raise ValueError("terminal delivery receipt claim digest is invalid")
    worktree, branch = receipt.get("worktree"), receipt.get("branch")
    if (not isinstance(worktree, str) or not pathlib.Path(worktree).is_absolute()
            or not isinstance(branch, str) or not branch):
        raise ValueError("terminal delivery receipt checkout provenance is invalid")
    try:
        observed_at = dt.datetime.fromisoformat(str(receipt.get("observed_at") or "").replace("Z", "+00:00"))
        if observed_at.tzinfo is None:
            raise ValueError("timezone required")
    except ValueError as exc:
        raise ValueError("terminal delivery receipt observation time is invalid") from exc

    merge = files["merge"]["record"]
    merge_digest = files["merge"]["digest"]
    expected_merge = {
        "receipt_type": "oasis7_pr_merge", "issuer": "github_live_query",
        "evidence_mode": "production", "repository": repository,
        "default_branch": receipt.get("default_branch"), "pr_number": pr_number,
        "pr_url": pr_url, "state": "MERGED", "head_oid": receipt.get("head_oid"),
        "base_ref": receipt.get("default_branch"),
    }
    if (any(merge.get(key) != value for key, value in expected_merge.items())
            or not merge.get("merged_at") or not merge.get("observed_at")):
        raise ValueError("terminal delivery merge receipt provenance mismatch")
    if merge_digest != receipt.get("merge_receipt_sha256"):
        raise ValueError("terminal delivery merge receipt digest mismatch")
    if merge.get("merge_commit_oid") not in (None, receipt.get("merge_commit_oid")):
        raise ValueError("terminal delivery merge receipt merge commit mismatch")
    if (task_record.get("merge_receipt") != merge
            or task_record.get("merge_receipt_sha256") != merge_digest):
        raise ValueError("terminal delivery task mapping merge receipt mismatch")

    try:
        from task_complete_claim import select_historical_task_complete_claim
        claim, claim_digest, _claim_comment = select_historical_task_complete_claim(
            repository, task_uid, task_record, live_issue, comments,
            accepted_head=receipt["head_oid"],
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise ValueError(f"terminal delivery accepted task_complete claim is missing, ambiguous, or invalid: {exc}") from exc
    if claim_digest != claim_hash:
        raise ValueError("terminal delivery accepted task_complete claim is missing, ambiguous, or invalid: digest mismatch")

    _validate_live_issue(live_issue, repository, task_uid, issue_number, pr_number, pr_url)
    _validate_live_pr(live_pr, repository, task_uid, issue_number, pr_number, pr_url,
                      receipt["head_oid"], receipt["merge_commit_oid"], receipt["default_branch"])
    target = _validate_live_repository(live_repository, repository, receipt["merge_commit_oid"],
                                       receipt["default_branch"], receipt["observed_target_oid"])
    project = _project_values(live_project_item)
    issue_url = f"https://github.com/{repository}/issues/{issue_number}"
    if (str(live_project_item.get("id")) != str(task_record.get("project_item_id") or "")
            or project["project_number"] != 1
            or project["project_owner"] != repository.split("/", 1)[0]
            or project["content"].get("number") != issue_number
            or project["content"].get("url") != issue_url
            or not _has_task_uid(project["content"].get("body"), task_uid)
            or any(project["fields"].get(key) != value for key, value in {
                "Status": "Done", "PM Status": "done", "Workflow Phase": "done",
            }.items())):
        raise ValueError("terminal delivery live Project item identity/phase mismatch")
    if project["fields"].get("Task UID", task_uid) != task_uid:
        raise ValueError("terminal delivery live Project Task UID mismatch")

    delivery_digest = files["delivery"]["digest"]
    expected_body = _v2_comment_body(receipt, delivery_digest)
    v2_comments, matches = [], []
    for comment in comments:
        body = str(comment.get("body") or "")
        if DELIVERY_MARKER in body:
            v2_comments.append(comment)
        if body == expected_body:
            comment_id = comment.get("id")
            if (type(comment_id) is int and comment_id > 0
                    and comment.get("html_url") == f"{issue_url}#issuecomment-{comment_id}"):
                matches.append(comment)
    if len(v2_comments) != 1 or len(matches) != 1:
        raise ValueError("terminal delivery comment readback mismatch: marker is missing, duplicate, or inconsistent")
    comment = matches[0]
    author = comment.get("user") or {}
    if (not isinstance(author, dict)
            or author.get("login") != repository.split("/", 1)[0]):
        raise ValueError("terminal v2 evidence comment author mismatch")
    comment_digest = hashlib.sha256(str(comment["body"]).encode("utf-8")).hexdigest()
    mapped_comment_id = (task_record.get("phase_receipt_comment_id") or {}).get("post_merge_done")
    mapped_comment_digest = (task_record.get("phase_receipt_comment_sha256") or {}).get("post_merge_done")
    if mapped_comment_id != comment.get("id") or mapped_comment_digest != comment_digest:
        raise ValueError("terminal delivery comment readback mismatch: mapping selector differs")

    ledger = files["ledger"]["record"]
    ledger_digest = files["ledger"]["digest"]
    operations = ledger.get("operations")
    if (ledger.get("schema") != "oasis7_finalizer_ledger_v1"
            or ledger.get("task_uid") != task_uid or not isinstance(operations, dict)):
        raise ValueError("terminal finalizer ledger or tombstone mismatch: ledger identity")
    for effect in ("project_update", "evidence_comment", "issue_close"):
        entry = operations.get(effect) or {}
        op_id = hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
        if (entry.get("effect"), entry.get("operation_id"), entry.get("committed")) != (effect, op_id, True):
            raise ValueError(f"terminal finalizer ledger or tombstone mismatch: {effect} is not committed")
    comment_result = (operations.get("evidence_comment") or {}).get("result") or {}
    if (comment_result.get("comment_id") != comment.get("id")
            or comment_result.get("comment_sha256") != comment_digest):
        raise ValueError("terminal finalizer ledger or tombstone mismatch: comment binding")
    project_result = (operations.get("project_update") or {}).get("result") or {}
    if any(project_result.get(k) != v for k, v in {
        "Status": "Done", "PM Status": "done", "Workflow Phase": "done",
    }.items()):
        raise ValueError("terminal finalizer ledger or tombstone mismatch: Project readback")
    issue_result = (operations.get("issue_close") or {}).get("result") or {}
    if (str(issue_result.get("state") or "").upper() != "CLOSED"
            or str(issue_result.get("state_reason", issue_result.get("stateReason", ""))).lower() != "completed"):
        raise ValueError("terminal finalizer ledger or tombstone mismatch: Issue readback")

    tombstone = files["tombstone"]["record"]
    tombstone_digest = files["tombstone"]["digest"]
    expected_tombstone = (
        "oasis7_terminal_tombstone_v1", task_uid, repository, issue_number,
        pr_number, "post_merge_done", delivery_digest, True, worktree, branch,
    )
    actual_tombstone = (
        tombstone.get("schema"), tombstone.get("task_uid"), tombstone.get("repository"),
        tombstone.get("issue_number"), tombstone.get("pr_number"), tombstone.get("workflow_phase"),
        tombstone.get("terminal_receipt_sha256"), tombstone.get("checkout_recreation_forbidden"),
        tombstone.get("canonical_worktree"), tombstone.get("task_branch"),
    )
    if actual_tombstone != expected_tombstone:
        raise ValueError("terminal finalizer ledger or tombstone mismatch: tombstone identity")
    return {
        "status": "passed", "protocol_version": 2, "task_uid": task_uid,
        "repository": repository, "issue_number": issue_number, "pr_number": pr_number,
        "pr_url": pr_url, "head_oid": receipt["head_oid"],
        "merge_commit_oid": receipt["merge_commit_oid"], "default_branch": receipt["default_branch"],
        "observed_target_oid": receipt["observed_target_oid"], "live_target_oid": target,
        "merge_receipt_sha256": merge_digest, "task_complete_claim_sha256": claim_digest,
        "readiness_proof_sha256": readiness["digest"],
        "terminal_receipt_sha256": delivery_digest, "delivery_receipt_sha256": delivery_digest,
        "comment_id": comment["id"], "comment_sha256": comment_digest,
        "finalizer_ledger_sha256": ledger_digest, "tombstone_sha256": tombstone_digest,
    }


def _v1_comment_matches(comments: list[dict], receipt: dict,
                        digests: dict[str, str]) -> tuple[dict, str]:
    repository = receipt.get("repository")
    issue_number = receipt.get("issue_number")
    pr_number = receipt.get("pr_number")
    pr_url = f"https://github.com/{repository}/pull/{pr_number}"
    issue_url = f"https://github.com/{repository}/issues/{issue_number}"
    matches = []
    for comment in comments:
        body = str(comment.get("body") or "").replace("\r\n", "\n")
        if "<!-- oasis7-pm-evidence -->" not in body:
            continue
        def plain(key: str) -> str | None:
            found = re.findall(rf"^{re.escape(key)}: ([^\n]+)$", body, re.MULTILINE)
            return found[0] if len(found) == 1 else None
        expected = {
            "Task UID": receipt.get("task_uid"), "Evidence Phase": "post_merge_done",
            "Receipt Chain Version": "1", "Receipt Type": "oasis7_terminal_cleanup",
            "Receipt Issuer": "post-merge-cleanup", "PR Number": str(pr_number), "PR URL": pr_url,
            "Merge Receipt SHA256": digests["merge"], "Main Sync Receipt SHA256": digests["main_sync"],
            "Terminal Receipt SHA256": digests["terminal"],
        }
        if any(plain(k) != v for k, v in expected.items()):
            continue
        chain = receipt_chain_digest(receipt.get("task_uid"), repository, issue_number,
                                     pr_number, pr_url, digests["merge"],
                                     digests["main_sync"], digests["terminal"])
        if plain("Receipt Chain Digest") != chain:
            continue
        op_id = hashlib.sha256(f"{receipt.get('task_uid')}:post_merge_done:evidence_comment".encode()).hexdigest()
        if plain("Operation-ID") != op_id:
            continue
        if (comment.get("user") or {}).get("login") != str(repository).split("/", 1)[0]:
            raise ValueError("terminal v1 evidence comment author mismatch")
        matches.append(comment)
    if len(matches) != 1:
        raise ValueError("terminal v1 evidence comment is missing or ambiguous")
    comment = matches[0]
    if (type(comment.get("id")) is not int or comment.get("id") <= 0
            or comment.get("html_url") != f"{issue_url}#issuecomment-{comment['id']}"):
        raise ValueError("terminal v1 evidence comment ID is invalid")
    return comment, hashlib.sha256(str(comment.get("body") or "").encode("utf-8")).hexdigest()


def read_terminal_proof(repo_root: pathlib.Path, task_uid: str, task_record: dict,
                        *, live_issue: dict, live_project_item: dict, live_pr: dict,
                        live_repository: dict, comments: list[dict]) -> dict:
    """Read one exact v1 or v2 terminal proof from canonical and live evidence."""
    if not isinstance(task_record, dict):
        raise ValueError("terminal delivery protocol selector mismatch: task mapping is malformed")
    type_map = task_record.get("phase_receipt_type") or {}
    digest_map = task_record.get("phase_receipt_sha256") or {}
    comment_id_map = task_record.get("phase_receipt_comment_id") or {}
    comment_digest_map = task_record.get("phase_receipt_comment_sha256") or {}
    phase_receipts = task_record.get("phase_receipts") or {}
    if not all(isinstance(x, dict) for x in (type_map, digest_map, comment_id_map,
                                             comment_digest_map, phase_receipts)):
        raise ValueError("terminal delivery protocol selector mismatch: mapped phase fields are malformed")
    version_type = type_map.get("post_merge_done")
    version_digest = digest_map.get("post_merge_done")
    has_v2_fields = any(
        isinstance(value, dict) and value.get("post_merge_done") is not None
        for value in (type_map, comment_id_map, comment_digest_map)
    )
    if version_type == "oasis7_terminal_delivery":
        comment_id = comment_id_map.get("post_merge_done")
        comment_digest = comment_digest_map.get("post_merge_done")
        if (not re_fullmatch_sha256(version_digest) or type(comment_id) is not int
                or comment_id <= 0 or not re_fullmatch_sha256(comment_digest)):
            raise ValueError("terminal delivery protocol selector mismatch: mapped v2 selector is incomplete")
        files = _read_v2_files(repo_root, task_uid)
        if files["delivery"]["digest"] != version_digest:
            raise ValueError("terminal delivery receipt digest mismatch: mapped digest differs from exact bytes")
        receipt = files["delivery"]["record"]
        result = _validate_delivery_record(receipt, files, task_uid, task_record,
                                           live_issue, live_project_item, live_pr,
                                           live_repository, comments)
        if (phase_receipts.get("post_merge_done") != receipt
                or result["comment_id"] != comment_id
                or result["comment_sha256"] != comment_digest):
            raise ValueError("terminal delivery protocol selector mismatch: mapped v2 projection differs")
        return result
    if has_v2_fields or version_type is not None:
        raise ValueError("terminal delivery protocol selector mismatch: unknown or mixed version selector")

    mapped_terminal = phase_receipts.get("post_merge_done")
    if not isinstance(mapped_terminal, dict) or mapped_terminal.get("receipt_type") != "oasis7_terminal_cleanup":
        raise ValueError("terminal delivery protocol selector mismatch: no exact v1 or v2 selector")
    if not re_fullmatch_sha256(version_digest):
        raise ValueError("terminal delivery protocol selector mismatch: mapped v1 digest is missing")
    receipts = read_receipt_chain(repo_root, task_uid)
    terminal = receipts["terminal"]["record"]
    if receipts["terminal"]["digest"] != version_digest or mapped_terminal != terminal:
        raise ValueError("terminal delivery protocol selector mismatch: mapped v1 receipt differs")
    repository = str(task_record.get("repository") or "")
    issue_number, pr_number = task_record.get("issue_number"), task_record.get("pr_number")
    pr_url = str(task_record.get("pr_url") or "")
    if type(issue_number) is not int or type(pr_number) is not int:
        raise ValueError("terminal delivery protocol selector mismatch: v1 mapping identity is invalid")
    digests = validate_receipt_chain(receipts, repository=repository, task_uid=task_uid,
                                     issue_number=issue_number, pr_number=pr_number,
                                     pr_url=pr_url, pr=live_pr)
    _validate_live_issue(live_issue, repository, task_uid, issue_number, pr_number, pr_url)
    project = _project_values(live_project_item)
    if (project["content"].get("number") != issue_number
            or project["content"].get("url") != f"https://github.com/{repository}/issues/{issue_number}"
            or not _has_task_uid(project["content"].get("body"), task_uid)
            or any(project["fields"].get(k) != v for k, v in {
                "Status": "Done", "PM Status": "done", "Workflow Phase": "done",
            }.items())):
        raise ValueError("terminal v1 live Project identity/phase mismatch")
    comment, comment_digest = _v1_comment_matches(comments, terminal, digests)
    return {
        "status": "passed", "protocol_version": 1, "task_uid": task_uid,
        "repository": repository, "issue_number": issue_number, "pr_number": pr_number,
        "pr_url": pr_url, "head_oid": receipts["merge"]["record"].get("head_oid"),
        "merge_commit_oid": live_pr.get("merge_commit_sha"),
        "default_branch": receipts["merge"]["record"].get("default_branch"),
        "observed_target_oid": receipts["main_sync"]["record"].get("remote_main_commit"),
        "merge_receipt_sha256": digests["merge"], "terminal_receipt_sha256": digests["terminal"],
        "comment_id": comment.get("id"), "comment_sha256": comment_digest,
        "finalizer_ledger_sha256": receipts["ledger"]["digest"],
        "tombstone_sha256": receipts["tombstone"]["digest"],
    }
