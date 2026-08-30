#!/usr/bin/env python3
"""Report the one supported next workflow command for a bound task.

GitHub Issue/Project truth is authoritative; the local mapping is a refreshed
cache and this command never mutates it.  Bootstrap snapshots, slice ledgers,
receipts, tombstones, and the durable supervisor checkpoint are read-only
evidence inputs used to detect stale or ambiguous state.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys
from typing import Any


TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
PHASES = {"intake", "bootstrap", "planning", "route", "execution", "verification",
          "blocked",
          "pre_pr_review", "pre_pr_ready", "pr_watch", "task_done",
          "main_sync", "closed_without_merge", "post_merge_done"}
STATUS_PHASES = {
    "candidate": {"", "intake", "bootstrap", "planning", "route"},
    "committed": {"", "execution", "verification", "pre_pr_review"},
    "blocked": {"", "blocked"},
    "ready": {"pre_pr_ready"},
    "pr_watch": {"pr_watch"},
    "done": {"task_done", "main_sync", "closed_without_merge", "post_merge_done"},
    "deferred": {"", "blocked"},
}
SNAPSHOT_SCHEMA = "oasis7.bootstrap-task-snapshot/v1"
CHECKPOINT_SCHEMA = "tpm-production-supervisor/v2"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", "--root", dest="repo_root", default=".")
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--mapping")
    parser.add_argument("--snapshot")
    parser.add_argument("--slice-ledger")
    parser.add_argument("--checkpoint")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def load_json(path: pathlib.Path) -> tuple[Any | None, str | None]:
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except (OSError, json.JSONDecodeError) as exc:
        return None, str(exc)


def add_blocker(blockers: list[str], message: str) -> None:
    if message not in blockers:
        blockers.append(message)


def git_value(path: pathlib.Path, *args: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(path), *args],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def registered_worktrees(root: pathlib.Path) -> list[tuple[pathlib.Path, str]]:
    entries: list[tuple[pathlib.Path, str]] = []
    current_path: pathlib.Path | None = None
    for line in (git_value(root, "worktree", "list", "--porcelain") + "\n").splitlines():
        if line.startswith("worktree "):
            current_path = pathlib.Path(line[len("worktree "):]).resolve()
        elif line.startswith("branch refs/heads/") and current_path is not None:
            entries.append((current_path, line[len("branch refs/heads/"):]))
        elif not line:
            current_path = None
    return entries


def canonical_default_worktree(
    root: pathlib.Path,
    task: dict[str, Any],
    blockers: list[str],
) -> pathlib.Path:
    default_branch = str(task.get("default_branch") or "")
    for path, branch in registered_worktrees(root):
        if branch == default_branch:
            return path
    add_blocker(blockers, "stale identity: canonical default worktree is unavailable")
    return root


def github_object_identity(url: str, kind: str) -> tuple[str, str] | None:
    if not url:
        return None
    match = re.search(
        rf"^https?://github\.com/([^/\s]+/[^/\s?#]+)/{kind}/(\d+)(?:$|[?#])",
        url,
        re.IGNORECASE,
    )
    return (match.group(1), match.group(2)) if match else None


def github_repository_identity(origin: str) -> str | None:
    """Return a GitHub owner/repository slug for supported origin forms."""
    if not origin:
        return None
    match = re.fullmatch(
        r"(?:https?://|ssh://git@|git@)github\.com[:/]([^/\s?#]+/[^/\s?#]+?)(?:\.git)?/?",
        origin.strip(),
        re.IGNORECASE,
    )
    return match.group(1) if match else None


def repository_identity(value: str) -> str | None:
    """Normalize a checkpoint repository value to the canonical GitHub slug."""
    if not value:
        return None
    if re.fullmatch(r"[^/\s]+/[^/\s]+", value):
        return value
    return github_repository_identity(value)


def verify_mapping_identity(
    root: pathlib.Path,
    task_uid: str,
    task: dict[str, Any],
    blockers: list[str],
) -> None:
    mapped_uid = str(task.get("task_uid") or "")
    if mapped_uid != task_uid:
        add_blocker(blockers, "stale identity: canonical mapping task UID does not match its key")

    repository = str(task.get("repository") or "")
    if not repository:
        return
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", repository):
        add_blocker(blockers, "stale identity: canonical mapping repository is malformed")

    canonical_text = str(task.get("canonical_worktree") or "")
    if not canonical_text:
        return
    canonical = pathlib.Path(canonical_text).expanduser().resolve()
    if canonical != root:
        add_blocker(blockers, "stale identity: canonical mapping worktree differs from query root")
    if not canonical.exists():
        add_blocker(blockers, "stale identity: canonical mapping worktree is unavailable")
        return
    live_root = git_value(canonical, "rev-parse", "--show-toplevel")
    if not live_root:
        add_blocker(blockers, "stale identity: canonical mapping worktree is not a Git worktree")
    elif pathlib.Path(live_root).resolve() != canonical:
        add_blocker(blockers, "stale identity: canonical mapping worktree resolves to a different repository")

    task_branch = str(task.get("task_branch") or "")
    live_branch = git_value(canonical, "symbolic-ref", "--quiet", "--short", "HEAD")
    if task_branch and live_branch != task_branch:
        add_blocker(
            blockers,
            f"stale identity: canonical mapping branch is {live_branch or 'detached'}, expected {task_branch}",
        )
    origin = git_value(canonical, "config", "--get", "remote.origin.url")
    origin_identity = github_repository_identity(origin)
    if not origin:
        add_blocker(blockers, "stale identity: canonical mapping local origin is missing")
    elif not origin_identity:
        add_blocker(blockers, "stale identity: canonical mapping local origin is unsupported (GitHub origin required)")
    elif origin_identity != repository:
        add_blocker(blockers, "stale identity: canonical mapping repository differs from local origin")

    issue_url = str(task.get("issue_url") or "")
    issue_identity = github_object_identity(issue_url, "issues")
    issue_number = str(task.get("issue_number") or "")
    if not issue_identity:
        add_blocker(blockers, "stale identity: task Issue URL is malformed or unsupported (GitHub Issue URL required)")
    elif issue_identity[0] != repository:
        add_blocker(blockers, "stale identity: task Issue URL belongs to a different repository")
    elif issue_number and issue_identity[1] != issue_number:
        add_blocker(blockers, "stale identity: task Issue URL does not match issue number")
    pr_identity = github_object_identity(
        str(task.get("pr_url") or task.get("pull_request_url") or ""),
        "pulls?",
    )
    pr_url = str(task.get("pr_url") or task.get("pull_request_url") or "")
    if pr_url and not pr_identity:
        add_blocker(blockers, "stale identity: task PR URL is malformed or unsupported (GitHub PR URL required)")
    elif pr_identity and pr_identity[0] != repository:
        add_blocker(blockers, "stale identity: task PR URL belongs to a different repository")


def verify_snapshot(
    path: pathlib.Path,
    root: pathlib.Path,
    mapping_path: pathlib.Path,
    task: dict[str, Any],
    blockers: list[str],
    sources: list[str],
    *,
    required: bool = False,
) -> None:
    if not path.exists():
        if required:
            add_blocker(blockers, "stale identity: bootstrap snapshot is missing after bootstrap")
        return
    sources.append(str(path))
    snapshot, error = load_json(path)
    if error or not isinstance(snapshot, dict):
        add_blocker(blockers, f"stale identity: bootstrap snapshot is unreadable ({error or 'not an object'})")
        return
    if snapshot.get("schema") != SNAPSHOT_SCHEMA:
        add_blocker(blockers, "stale identity: bootstrap snapshot schema is unsupported")
    if not isinstance(snapshot.get("producer"), str) or not snapshot.get("producer"):
        add_blocker(blockers, "stale identity: bootstrap snapshot producer is missing")
    if not isinstance(snapshot.get("created_at"), str) or not snapshot.get("created_at"):
        add_blocker(blockers, "stale identity: bootstrap snapshot creation time is missing")
    request = snapshot.get("request")
    request_identity = request.get("identity") if isinstance(request, dict) else ""
    if not isinstance(request_identity, str) or not request_identity:
        add_blocker(blockers, "stale identity: bootstrap snapshot request identity is missing")
        return
    validator = pathlib.Path(__file__).with_name("bootstrap-task-snapshot.py")
    result = subprocess.run(
        [sys.executable, str(validator), "validate-epoch-identity",
         "--repo-root", str(root), "--tasks-json", str(mapping_path),
         "--snapshot", str(path), "--task-uid", str(task.get("task_uid") or ""),
         "--request-identity", request_identity],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        detail = result.stderr.strip().splitlines()[-1] if result.stderr.strip() else "validator rejected snapshot"
        add_blocker(blockers, f"stale identity: bootstrap snapshot validator rejected snapshot ({detail})")


def verify_ledger(
    path: pathlib.Path,
    task_uid: str,
    blockers: list[str],
    sources: list[str],
) -> None:
    if not path.exists():
        return
    sources.append(str(path))
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        add_blocker(blockers, f"stale identity: slice ledger is unreadable ({exc})")
        return
    for line_number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            add_blocker(blockers, f"ambiguous state: slice ledger line {line_number} is not JSON")
            continue
        if not isinstance(entry, dict) or entry.get("task_uid") != task_uid:
            add_blocker(blockers, f"stale identity: slice ledger line {line_number} has a different task UID")
        elif any(not str(entry.get(key) or "") for key in ("role", "status", "head")):
            add_blocker(blockers, f"stale identity: slice ledger line {line_number} lacks role/status/head authority")


def verify_checkpoint(
    path: pathlib.Path,
    root: pathlib.Path,
    task: dict[str, Any],
    blockers: list[str],
    sources: list[str],
) -> dict[str, Any] | None:
    if not path.exists():
        return None
    sources.append(str(path))
    checkpoint, error = load_json(path)
    if error or not isinstance(checkpoint, dict):
        add_blocker(blockers, f"stale identity: durable workflow checkpoint is unreadable ({error or 'not an object'})")
        return None
    if checkpoint.get("schema") != CHECKPOINT_SCHEMA:
        add_blocker(blockers, "stale identity: durable workflow checkpoint schema is unsupported")
    if type(checkpoint.get("revision")) is not int or checkpoint.get("revision") < 1:
        add_blocker(blockers, "stale identity: durable workflow checkpoint revision is missing or invalid")
    for key in ("phase", "status", "capability_status"):
        if not isinstance(checkpoint.get(key), str) or not checkpoint.get(key):
            add_blocker(blockers, f"stale identity: durable workflow checkpoint {key} is missing")
    if checkpoint.get("task_uid") != task.get("task_uid"):
        add_blocker(blockers, "stale identity: durable workflow checkpoint task UID drift")
    if checkpoint.get("repo") in (None, ""):
        add_blocker(blockers, "stale identity: durable workflow checkpoint repository is missing")
    elif pathlib.Path(str(checkpoint["repo"])).resolve() != root:
        add_blocker(blockers, "stale identity: durable workflow checkpoint repository drift")
    checkpoint_repository = str(checkpoint.get("repository") or "")
    checkpoint_repository_identity = repository_identity(checkpoint_repository)
    task_repository = str(task.get("repository") or "")
    if not checkpoint_repository:
        add_blocker(blockers, "stale identity: durable workflow checkpoint remote repository is missing")
    elif not checkpoint_repository_identity:
        add_blocker(blockers, "stale identity: durable workflow checkpoint remote repository is unsupported")
    elif checkpoint_repository_identity != task_repository:
        add_blocker(blockers, "stale identity: durable workflow checkpoint remote repository drift")
    if checkpoint.get("state") not in (None, "") and pathlib.Path(str(checkpoint["state"])).resolve() != path:
        add_blocker(blockers, "stale identity: durable workflow checkpoint path drift")
    terminal = checkpoint.get("terminal_authority")
    if terminal is None:
        add_blocker(blockers, "stale identity: durable workflow checkpoint terminal authority is missing")
    elif not isinstance(terminal, dict):
        add_blocker(blockers, "stale identity: durable workflow checkpoint terminal authority is malformed")
    else:
        for key, expected in (
            ("task_uid", task.get("task_uid")),
            ("repository", task_repository),
            ("canonical_worktree", task.get("canonical_worktree")),
            ("task_branch", task.get("task_branch")),
            ("default_branch", task.get("default_branch")),
        ):
            actual = terminal.get(key)
            if actual in (None, ""):
                add_blocker(blockers, f"stale identity: durable workflow checkpoint terminal {key} is missing")
            elif key == "canonical_worktree":
                if pathlib.Path(str(actual)).resolve() != pathlib.Path(str(expected)).resolve():
                    add_blocker(blockers, "stale identity: durable workflow checkpoint terminal worktree drift")
            elif key == "repository":
                terminal_identity = repository_identity(str(actual))
                if not terminal_identity:
                    add_blocker(blockers, "stale identity: durable workflow checkpoint terminal repository is unsupported")
                elif terminal_identity != task_repository:
                    add_blocker(blockers, "stale identity: durable workflow checkpoint terminal repository drift")
                elif checkpoint_repository_identity and terminal_identity != checkpoint_repository_identity:
                    add_blocker(blockers, "stale identity: durable workflow checkpoint terminal repository drift")
            elif str(actual) != str(expected):
                add_blocker(blockers, f"stale identity: durable workflow checkpoint terminal {key} drift")
    return checkpoint


def canonical_receipt_root(root: pathlib.Path, task_uid: str,
                           blockers: list[str], sources: list[str]) -> pathlib.Path | None:
    raw_common = git_value(root, "rev-parse", "--git-common-dir")
    if not raw_common:
        add_blocker(blockers, "stale identity: cannot resolve canonical Git common directory for terminal proof")
        return None
    common = pathlib.Path(raw_common)
    if not common.is_absolute():
        common = root / common
    receipt_root = common.resolve() / "oasis7-workflow-receipts" / task_uid
    return receipt_root


def verify_bound_json_file(
    path: pathlib.Path,
    expected_digest: object,
    expected_object: object,
    label: str,
    blockers: list[str],
    sources: list[str],
) -> dict[str, Any] | None:
    sources.append(str(path))
    if not isinstance(expected_digest, str) or not SHA256_RE.fullmatch(expected_digest):
        add_blocker(blockers, f"stale identity: terminal {label} digest authority is missing or malformed")
        return None
    if not path.is_file():
        add_blocker(blockers, f"stale identity: terminal {label} proof is missing")
        return None
    try:
        raw = path.read_bytes()
        value = json.loads(raw.decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        add_blocker(blockers, f"stale identity: terminal {label} proof is unreadable ({exc})")
        return None
    actual_digest = hashlib.sha256(raw).hexdigest()
    if actual_digest != expected_digest:
        add_blocker(blockers, f"stale identity: terminal {label} digest mismatch")
    if expected_object is not None and value != expected_object:
        add_blocker(blockers, f"stale identity: terminal {label} differs from task mapping authority")
    if not isinstance(value, dict):
        add_blocker(blockers, f"stale identity: terminal {label} is not an object")
        return None
    return value


def verify_terminal_proof(
    root: pathlib.Path,
    phase: str,
    task: dict[str, Any],
    blockers: list[str],
    sources: list[str],
) -> None:
    """Require durable proof before exposing a terminal command or completion."""
    if phase not in {"task_done", "main_sync", "closed_without_merge", "post_merge_done"}:
        return
    receipt_root = canonical_receipt_root(root, str(task.get("task_uid") or ""), blockers, sources)
    if receipt_root is None:
        return

    phase_receipts = task.get("phase_receipts")
    phase_receipts = phase_receipts if isinstance(phase_receipts, dict) else {}
    phase_digests = task.get("phase_receipt_sha256")
    phase_digests = phase_digests if isinstance(phase_digests, dict) else {}

    def require_phase_receipt(name: str, filenames: tuple[str, ...]) -> dict[str, Any] | None:
        expected = phase_receipts.get(name)
        digest = phase_digests.get(name)
        candidates = [receipt_root / filename for filename in filenames]
        for candidate in candidates:
            if candidate.is_file():
                return verify_bound_json_file(candidate, digest, expected,
                                              f"{name.replace('_', '-')} receipt", blockers, sources)
        # Keep the missing-proof message deterministic while still checking
        # the digest contract against the canonical candidate path.
        return verify_bound_json_file(candidates[0], digest, expected,
                                      f"{name.replace('_', '-')} receipt", blockers, sources)

    if phase == "task_done":
        if str(task.get("completion_mode") or "") == "non_pr_task":
            evidence_file = pathlib.Path(str(task.get("non_pr_completion_evidence_file") or ""))
            expected_digest = task.get("non_pr_completion_evidence_sha256")
            if not evidence_file.is_absolute() or evidence_file.resolve() != (
                    root / ".pm/scratch" / str(task.get("task_uid") or "") /
                    "non-pr-completion-evidence.txt").resolve():
                add_blocker(blockers, "stale identity: non-PR completion evidence path is not canonical")
            if not evidence_file.is_file():
                add_blocker(blockers, "stale identity: non-PR completion evidence proof is missing")
            else:
                raw = evidence_file.read_bytes()
                sources.append(str(evidence_file))
                if not isinstance(expected_digest, str) or not SHA256_RE.fullmatch(expected_digest):
                    add_blocker(blockers, "stale identity: non-PR completion evidence digest is missing or malformed")
                elif hashlib.sha256(raw).hexdigest() != expected_digest:
                    add_blocker(blockers, "stale identity: non-PR completion evidence digest mismatch")
                expected_raw = (str(task.get("non_pr_completion_evidence") or "") + "\n").encode("utf-8")
                if raw != expected_raw:
                    add_blocker(blockers, "stale identity: non-PR completion evidence content drift")
        else:
            merge = task.get("merge_receipt")
            if not isinstance(merge, dict):
                add_blocker(blockers, "stale identity: merge receipt authority is missing")
            else:
                receipt = verify_bound_json_file(
                    receipt_root / "merge-receipt.json", task.get("merge_receipt_sha256"),
                    merge, "merge", blockers, sources,
                )
                if receipt is not None and (
                    receipt.get("task_uid") != task.get("task_uid")
                    or receipt.get("repository") != task.get("repository")
                ):
                    add_blocker(blockers, "stale identity: merge receipt task/repository identity drift")
        return

    if phase in {"main_sync", "post_merge_done"}:
        merge = task.get("merge_receipt")
        if not isinstance(merge, dict):
            add_blocker(blockers, "stale identity: terminal receipt chain lacks merge receipt authority")
        else:
            receipt = verify_bound_json_file(
                receipt_root / "merge-receipt.json", task.get("merge_receipt_sha256"),
                merge, "merge", blockers, sources,
            )
            if receipt is not None and (
                    receipt.get("task_uid") != task.get("task_uid")
                    or receipt.get("repository") != task.get("repository")):
                add_blocker(blockers, "stale identity: merge receipt task/repository identity drift")
    if phase in {"main_sync", "post_merge_done"}:
        sync = require_phase_receipt("main_sync", ("main-sync-receipt.json",))
        if sync is not None and (
                sync.get("task_uid") != task.get("task_uid")
                or sync.get("repository") != task.get("repository")):
            add_blocker(blockers, "stale identity: main-sync receipt task/repository identity drift")
    if phase in {"closed_without_merge", "post_merge_done"}:
        terminal_name = "closed_without_merge" if phase == "closed_without_merge" else "post_merge_done"
        filenames = (("closed-without-merge-receipt.json",
                      "closed-without-merge-receipt-migrated.json")
                     if phase == "closed_without_merge"
                     else ("terminal-cleanup-receipt.json",))
        terminal = require_phase_receipt(terminal_name, filenames)
        if terminal is not None:
            if (terminal.get("task_uid") != task.get("task_uid")
                    or terminal.get("repository") != task.get("repository")):
                add_blocker(blockers, "stale identity: terminal receipt task/repository identity drift")
            expected_worktree = pathlib.Path(str(task.get("canonical_worktree") or "")).resolve()
            receipt_worktree = terminal.get("worktree")
            if not receipt_worktree:
                add_blocker(blockers, "stale identity: terminal receipt worktree identity is missing")
            elif pathlib.Path(str(receipt_worktree)).expanduser().resolve() != expected_worktree:
                add_blocker(blockers, "stale identity: terminal receipt worktree identity drift")
            if not terminal.get("branch"):
                add_blocker(blockers, "stale identity: terminal receipt branch identity is missing")
            elif str(terminal.get("branch")) != str(task.get("task_branch") or ""):
                add_blocker(blockers, "stale identity: terminal receipt branch identity drift")
            tombstone_path = receipt_root / "terminal-tombstone.json"
            sources.append(str(tombstone_path))
            tombstone, error = load_json(tombstone_path)
            if error or not isinstance(tombstone, dict):
                add_blocker(blockers, f"stale identity: terminal tombstone is unreadable ({error or 'not an object'})")
            else:
                if tombstone.get("schema") != "oasis7_terminal_tombstone_v1":
                    add_blocker(blockers, "stale identity: terminal tombstone schema is unsupported")
                if tombstone.get("task_uid") != task.get("task_uid"):
                    add_blocker(blockers, "stale identity: terminal tombstone task UID drift")
                if tombstone.get("repository") != task.get("repository"):
                    add_blocker(blockers, "stale identity: terminal tombstone repository drift")
                if tombstone.get("workflow_phase") != phase:
                    add_blocker(blockers, "stale identity: terminal tombstone phase drift")
                if tombstone.get("terminal_receipt_sha256") != phase_digests.get(terminal_name):
                    add_blocker(blockers, "stale identity: terminal tombstone digest drift")
                if tombstone.get("checkout_recreation_forbidden") is not True:
                    add_blocker(blockers, "stale identity: terminal tombstone lacks cleanup prohibition")
            ledger_names = (("non-merge-finalizer-ledger.json",
                             "non-merge-finalizer-ledger-migrated.json")
                            if phase == "closed_without_merge"
                            else ("finalizer-ledger.json",))
            ledger_path = next((receipt_root / name for name in ledger_names
                                if (receipt_root / name).is_file()), receipt_root / ledger_names[0])
            ledger = load_json(ledger_path)[0] if ledger_path.is_file() else None
            sources.append(str(ledger_path))
            if not isinstance(ledger, dict):
                add_blocker(blockers, "stale identity: terminal finalizer ledger is missing or unreadable")
            else:
                if ledger.get("task_uid") != task.get("task_uid"):
                    add_blocker(blockers, "stale identity: terminal finalizer ledger task UID drift")
                if ledger.get("schema") not in {"oasis7_non_merge_finalizer_ledger_v1",
                                                  "oasis7_finalizer_ledger_v1"}:
                    add_blocker(blockers, "stale identity: terminal finalizer ledger schema is unsupported")
                if type(ledger.get("revision")) is not int or ledger.get("revision") < 1:
                    add_blocker(blockers, "stale identity: terminal finalizer ledger revision is missing or invalid")
                operations = ledger.get("operations")
                if (not isinstance(operations, dict)
                        or not any(isinstance(entry, dict) and entry.get("committed") is True
                                   for entry in operations.values())):
                    add_blocker(blockers, "stale identity: terminal finalizer ledger has no committed operation proof")


def command_for(
    phase: str,
    task: dict[str, Any],
    root: pathlib.Path,
    blockers: list[str],
    default_root: pathlib.Path | None = None,
) -> list[str]:
    uid = str(task.get("task_uid") or "")
    status = str(task.get("status") or "")
    pr = str(task.get("pr_number") or "")
    pr_url = str(task.get("pr_url") or task.get("pull_request_url") or "")
    if phase in {"intake", "planning", "route", "blocked"}:
        return []
    if phase in {"closed_without_merge", "post_merge_done"}:
        return []
    if phase == "bootstrap":
        return [
            "python3", "scripts/pm/bootstrap-task-snapshot.py", "validate-or-create",
            "--repo-root", str(task.get("canonical_worktree") or root), "--task-uid", uid,
            "--producer", "human-operated-tpm",
        ]
    if phase == "execution":
        return ["./scripts/pm/github-project-workflow.sh", "--json", "audit", "--task-uid", uid]
    if phase in {"verification", "pre_pr_review", "pre_pr_ready"}:
        return ["./scripts/prepare-task-pr.sh", "--json"]
    if phase == "pr_watch":
        if not pr or not pr.isdigit() or not pr_url:
            add_blocker(blockers, "ambiguous state: PR watch phase lacks bound PR identity")
            return []
        return ["python3", "./scripts/pm/pr-lifecycle-gate.py", pr, "--task-uid", uid, "--json"]
    if phase == "task_done":
        terminal_root = default_root or root
        if status == "done" and task.get("completion_mode") == "non_pr_task":
            evidence_path = str(task.get("non_pr_completion_evidence_file") or
                                root / ".pm/scratch" / uid / "non-pr-completion-evidence.txt")
            if not pathlib.Path(evidence_path).is_file():
                add_blocker(blockers, "ambiguous state: non-PR completion evidence file is unavailable")
                return []
            return [
                "python3", "./scripts/pm/non-merge-finalize.py", "--repo-root", str(terminal_root),
                "--task-uid", uid, "--reason", "non_pr_completed", "--evidence-file", evidence_path, "--json",
            ]
        if not pr or not pr.isdigit() or not pr_url:
            add_blocker(blockers, "ambiguous state: merged terminal phase lacks bound PR identity")
            return []
        return [
            "./scripts/pm/finalize-task.sh", "--repo-root", str(terminal_root), "--task-uid", uid,
            "--pr", pr, "--resume", "--json",
        ]
    if phase == "main_sync":
        if not pr or not pr.isdigit() or not pr_url:
            add_blocker(blockers, "ambiguous state: main-sync phase lacks bound PR identity")
            return []
        terminal_root = default_root or root
        return ["./scripts/pm/finalize-task.sh", "--repo-root", str(terminal_root), "--task-uid", uid, "--pr", pr, "--resume", "--json"]
    add_blocker(blockers, f"ambiguous state: unsupported workflow phase {phase!r}")
    return []


def main() -> int:
    args = parse_args()
    root = pathlib.Path(args.repo_root).resolve()
    mapping_path = pathlib.Path(args.mapping).resolve() if args.mapping else root / ".pm/github-project-sync/tasks.json"
    payload: dict[str, Any] = {
        "task_uid": args.task_uid,
        "workflow_phase": "",
        "identity_status": "blocked",
        "evidence_sources": [str(mapping_path)],
        "blockers": [],
        "next_command": [],
        "next_action": "blocked",
    }
    blockers = payload["blockers"]
    if not TASK_UID_RE.fullmatch(args.task_uid):
        add_blocker(blockers, "stale identity: invalid task UID")
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 1
    mapping, error = load_json(mapping_path)
    if error or not isinstance(mapping, dict):
        add_blocker(blockers, f"stale identity: canonical task mapping is unreadable ({error or 'not an object'})")
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 1
    task = (mapping.get("tasks") or {}).get(args.task_uid)
    if not isinstance(task, dict):
        add_blocker(blockers, "stale identity: task UID is absent from canonical mapping")
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 1
    task = dict(task)
    verify_mapping_identity(root, args.task_uid, task, blockers)
    payload["task_uid"] = args.task_uid
    payload.update({
        "status": str(task.get("status") or ""),
        "repository": str(task.get("repository") or ""),
        "issue_number": task.get("issue_number"),
        "issue_url": str(task.get("issue_url") or ""),
        "canonical_worktree": str(task.get("canonical_worktree") or ""),
        "task_branch": str(task.get("task_branch") or ""),
        "default_branch": str(task.get("default_branch") or ""),
    })
    for key in ("repository", "issue_number", "issue_url", "canonical_worktree", "task_branch", "default_branch"):
        if payload[key] in (None, ""):
            add_blocker(blockers, f"stale identity: task truth missing {key}")
    raw_phase = str(task.get("workflow_phase") or "")
    status = str(task.get("status") or "")
    pr_number = str(task.get("pr_number") or "")
    pr_url = str(task.get("pr_url") or task.get("pull_request_url") or "")
    if pr_url:
        pr_identity = github_object_identity(pr_url, "pulls?")
        if not pr_identity:
            add_blocker(blockers, "stale identity: task PR URL is malformed or unsupported (GitHub PR URL required)")
        elif pr_identity[0] != str(task.get("repository") or ""):
            add_blocker(blockers, "stale identity: task PR URL belongs to a different repository")
        pr_match = re.search(r"/pulls?/(\d+)(?:$|[?#])", pr_url)
        if not pr_match:
            add_blocker(blockers, "stale identity: task PR URL is malformed")
        elif pr_number and pr_match.group(1) != pr_number:
            add_blocker(blockers, "stale identity: task PR URL does not match PR number")
    if raw_phase not in PHASES and raw_phase != "":
        add_blocker(blockers, f"ambiguous state: unsupported workflow phase {raw_phase!r}")
        phase = raw_phase
    else:
        phase = raw_phase
        if not phase:
            phase = {"candidate": "bootstrap", "committed": "execution", "ready": "pre_pr_ready",
                     "pr_watch": "pr_watch", "done": "task_done", "deferred": "blocked",
                     "blocked": "blocked"}.get(status, "")
        if status not in STATUS_PHASES:
            add_blocker(blockers, f"ambiguous state: unsupported task status {status!r}")
        elif raw_phase not in STATUS_PHASES[status]:
            add_blocker(blockers, f"ambiguous state: status {status!r} cannot use workflow phase {raw_phase!r}")
    payload["workflow_phase"] = phase
    if phase not in {"intake", "bootstrap"} and not str(task.get("cache_refreshed_at") or ""):
        add_blocker(blockers, "stale identity: task mapping lacks live GitHub refresh evidence")

    snapshot_path = pathlib.Path(args.snapshot).resolve() if args.snapshot else root / ".pm/scratch" / args.task_uid / "bootstrap-task-snapshot.json"
    ledger_path = pathlib.Path(args.slice_ledger).resolve() if args.slice_ledger else root / ".pm/scratch" / args.task_uid / "slice-ledger.jsonl"
    checkpoint_path = pathlib.Path(args.checkpoint).resolve() if args.checkpoint else root / ".pm/tasks" / f"{args.task_uid}.workflow.json"
    sources = payload["evidence_sources"]
    verify_snapshot(snapshot_path, root, mapping_path, task, blockers, sources,
                    required=phase not in {"intake", "bootstrap"})
    verify_ledger(ledger_path, args.task_uid, blockers, sources)
    checkpoint = verify_checkpoint(checkpoint_path, root, task, blockers, sources)
    if checkpoint and checkpoint.get("phase") not in (None, "", phase):
        add_blocker(blockers, "ambiguous state: durable workflow checkpoint phase disagrees with task mapping")
    verify_terminal_proof(root, phase, task, blockers, sources)
    payload["evidence_sources"] = list(dict.fromkeys(sources))
    default_root = None
    if phase in {"task_done", "main_sync"}:
        default_root = canonical_default_worktree(root, task, blockers)
    if not blockers:
        payload["next_command"] = command_for(phase, task, root, blockers, default_root)
        payload["next_action"] = "run_command" if payload["next_command"] else (
            "completed" if phase in {"closed_without_merge", "post_merge_done"} else "action_required"
        )
    if blockers:
        payload["next_command"] = []
        payload["identity_status"] = "stale" if any(item.startswith("stale identity") for item in blockers) else "ambiguous"
    else:
        payload["identity_status"] = "bound"
    if payload["next_command"]:
        payload["command_cwd"] = str(default_root if phase in {"task_done", "main_sync"} else root)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 1 if blockers else 0


if __name__ == "__main__":
    raise SystemExit(main())
