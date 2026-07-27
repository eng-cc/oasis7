#!/usr/bin/env python3
"""Start a new bootstrap epoch when one task UID must change branch identity.

This is intentionally the only PM writer that may replace a task record's
active worktree/branch identity.  It keeps the mapping lock while Git creates
the replacement and writes the new record only after every identity readback
has succeeded.
"""
from __future__ import annotations

import argparse
import base64
import contextlib
import datetime as dt
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import urllib.parse
from typing import Any, Iterator

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
_store_spec = importlib.util.spec_from_file_location("workflow_durable_store", SCRIPT_DIR / "workflow-durable-store.py")
assert _store_spec and _store_spec.loader
durable_store = importlib.util.module_from_spec(_store_spec)
_store_spec.loader.exec_module(durable_store)


class MigrationError(Exception):
    pass


JOURNAL_SCHEMA = "oasis7_branch_identity_migration/v1"
REMOTE_QUERY_TIMEOUT_SECONDS = 15


def git_executable() -> str:
    executable = shutil.which("git")
    if executable is None and sys.platform == "win32":
        candidate = pathlib.Path("C:/Program Files/Git/cmd/git.exe")
        if candidate.is_file():
            executable = str(candidate)
    if executable is None:
        raise MigrationError("git executable not found")
    return executable


def git(root: pathlib.Path, *args: str) -> str:
    result = subprocess.run([git_executable(), "-C", str(root), *args], text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise MigrationError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def git_ok(root: pathlib.Path, *args: str) -> bool:
    return subprocess.run([git_executable(), "-C", str(root), *args], stdout=subprocess.DEVNULL,
                          stderr=subprocess.DEVNULL).returncode == 0


def digest(value: dict[str, Any]) -> str:
    unsigned = dict(value)
    unsigned.pop("digest", None)
    data = json.dumps(unsigned, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return "sha256:" + hashlib.sha256(data).hexdigest()


def path_common_dir(root: pathlib.Path) -> pathlib.Path:
    value = pathlib.Path(git(root, "rev-parse", "--git-common-dir"))
    return (root / value).resolve() if not value.is_absolute() else value.resolve()


def worktree_branches(root: pathlib.Path) -> dict[pathlib.Path, str]:
    entries: dict[pathlib.Path, str] = {}
    current: pathlib.Path | None = None
    for line in git(root, "worktree", "list", "--porcelain").splitlines():
        if line.startswith("worktree "):
            current = pathlib.Path(line[9:]).resolve()
        elif current is not None and line.startswith("branch "):
            entries[current] = line[7:]
    return entries


def positive_integer(value: Any, label: str) -> int:
    if type(value) is not int or value < 1:
        raise MigrationError(f"{label} must be a positive integer")
    return value


def load_journal(path: pathlib.Path, task_uid: str) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        journal = durable_store.recover_atomic_journal(path)
    except (OSError, json.JSONDecodeError, TypeError, ValueError) as exc:
        raise MigrationError(f"cannot read migration journal {path}: {exc}") from exc
    if not isinstance(journal, dict):
        raise MigrationError("migration journal root must be an object")
    if journal.get("schema") != JOURNAL_SCHEMA:
        raise MigrationError("migration journal schema mismatch")
    if journal.get("task_uid") != task_uid:
        raise MigrationError("migration journal task UID mismatch")
    positive_integer(journal.get("revision"), "journal revision")
    return journal


def normalized_repository_identity(value: str) -> str:
    identity = value.strip().strip("/")
    if identity.endswith(".git"):
        identity = identity[:-4]
    parts = identity.split("/")
    if len(parts) != 2 or any(not part or any(char.isspace() for char in part) for part in parts):
        raise MigrationError(f"invalid repository identity: {value!r}")
    return "/".join(parts)


def test_remote_repository_map() -> dict[str, str]:
    raw = os.environ.get("OASIS7_PM_TEST_REMOTE_REPOSITORY_MAP")
    if not raw:
        return {}
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise MigrationError(f"test remote repository identity map is invalid: {exc}") from exc
    if not isinstance(value, dict) or not all(isinstance(key, str) and isinstance(item, str)
                                              for key, item in value.items()):
        raise MigrationError("test remote repository identity map must be a string-to-string object")
    return {key: normalized_repository_identity(item) for key, item in value.items()}


def remote_repository_identity(url: str, test_map: dict[str, str]) -> str:
    candidates = [url]
    path = pathlib.Path(url).expanduser()
    if path.is_absolute() or path.exists():
        candidates.append(str(path.resolve()))
    for candidate in candidates:
        if candidate in test_map:
            return test_map[candidate]

    repository_path = ""
    if "://" in url:
        parsed = urllib.parse.urlparse(url)
        if parsed.scheme not in {"http", "https", "ssh", "git"} or not parsed.netloc:
            raise MigrationError(f"remote repository identity is uncertain for URL {url!r}")
        repository_path = parsed.path
    else:
        scp = re.fullmatch(r"(?:[^@/:]+@)?[^/:]+:(.+)", url)
        if scp:
            repository_path = scp.group(1)
    if not repository_path:
        raise MigrationError(f"remote repository identity is uncertain for URL {url!r}")
    parts = repository_path.strip("/").split("/")
    if len(parts) < 2:
        raise MigrationError(f"remote repository identity is uncertain for URL {url!r}")
    return normalized_repository_identity("/".join(parts[-2:]))


def authoritative_remotes(root: pathlib.Path, repository: str) -> tuple[str, list[str]]:
    authoritative_repository = normalized_repository_identity(repository)
    remotes = [name for name in git(root, "remote").splitlines() if name]
    if not remotes:
        raise MigrationError("capability_blocked: no configured authoritative remote")
    identity_map = test_remote_repository_map()
    matched: list[str] = []
    for remote in remotes:
        urls = [url for url in git(root, "remote", "get-url", "--all", remote).splitlines() if url]
        if not urls:
            raise MigrationError(f"capability_blocked: remote repository identity is uncertain for {remote}")
        identities = {remote_repository_identity(url, identity_map) for url in urls}
        if len(identities) != 1:
            raise MigrationError(f"capability_blocked: remote repository identity is uncertain for {remote}")
        identity = identities.pop()
        if identity.casefold() == authoritative_repository.casefold():
            matched.append(remote)
    if not matched:
        raise MigrationError(
            "capability_blocked: repository identity mismatch; no authoritative remote matches task repository"
        )
    return authoritative_repository, matched


def live_remote_branch_collision(root: pathlib.Path, branch: str,
                                 remotes: list[str]) -> tuple[bool, list[str]]:
    collisions: list[str] = []
    for remote in remotes:
        try:
            result = subprocess.run(
                [git_executable(), "-C", str(root), "ls-remote", "--exit-code", "--heads",
                 remote, branch_ref(branch)],
                text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                timeout=REMOTE_QUERY_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired as exc:
            raise MigrationError(
                f"remote branch uniqueness is uncertain for {remote}: query timed out"
            ) from exc
        if result.returncode == 0:
            collisions.append(remote)
        elif result.returncode != 2:
            detail = result.stderr.strip() or f"git ls-remote exited {result.returncode}"
            raise MigrationError(f"remote branch uniqueness is uncertain for {remote}: {detail}")
    return bool(collisions), collisions


@contextlib.contextmanager
def mapping_lock(path: pathlib.Path) -> Iterator[dict[str, Any]]:
    """Lock without rewriting the mapping unless the caller explicitly commits."""
    path = path.resolve()
    lock = durable_store.mapping_lock_path(path)
    lock.parent.mkdir(parents=True, exist_ok=True)
    with lock.open("a+b") as handle:
        durable_store.ensure_lock_byte(handle)
        durable_store.fcntl.flock(handle.fileno(), durable_store.fcntl.LOCK_EX)
        try:
            yield json.loads(path.read_text(encoding="utf-8"))
        finally:
            durable_store.fcntl.flock(handle.fileno(), durable_store.fcntl.LOCK_UN)


def journal_path(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    return root / ".pm" / "scratch" / task_uid / "branch-identity-migration.json"


def update_journal(path: pathlib.Path, patch: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    def update(current: dict[str, Any]) -> None:
        if current:
            if current.get("schema") != JOURNAL_SCHEMA:
                raise MigrationError("migration journal schema mismatch")
            if current.get("task_uid") != patch["task_uid"]:
                raise MigrationError("migration journal task UID mismatch")
            positive_integer(current.get("revision"), "journal revision")
        current.update(patch)
        current.setdefault("schema", JOURNAL_SCHEMA)
        current.setdefault("revision", 1)
        positive_integer(current["revision"], "journal revision")
        result.update(current)
    durable_store.transact_json(path, update, {})
    return result


def require_record(mapping: dict[str, Any], task_uid: str) -> dict[str, Any]:
    records = mapping.get("tasks")
    record = records.get(task_uid) if isinstance(records, dict) else None
    if not isinstance(record, dict) or record.get("task_uid") != task_uid:
        raise MigrationError("missing or ambiguous task mapping")
    required = ("repository", "issue_number", "project_item_id", "owner_role", "default_branch",
                "canonical_worktree", "task_branch", "status")
    missing = [field for field in required if record.get(field) in (None, "")]
    if missing:
        raise MigrationError("task mapping is missing " + ", ".join(missing))
    if record.get("status") == "done":
        raise MigrationError("terminal task cannot start a new bootstrap epoch")
    return record


def branch_ref(branch: str) -> str:
    if not branch or branch.startswith("-") or ".." in branch or branch.endswith("/"):
        raise MigrationError("replacement branch name is invalid")
    return "refs/heads/" + branch


def replacement_is_proven(root: pathlib.Path, replacement: pathlib.Path, branch: str,
                          head: str, journal: dict[str, Any]) -> bool:
    return (journal.get("state") in {"replacement_branch_created", "committed"}
            and journal.get("replacement_branch") == branch
            and journal.get("replacement_worktree") == str(replacement)
            and journal.get("implementation_head") == head
            and replacement.is_dir()
            and git_ok(root, "rev-parse", "--verify", branch_ref(branch) + "^{commit}")
            and git(root, "rev-parse", branch_ref(branch)) == head
            and git(replacement, "rev-parse", "HEAD") == head
            and git(replacement, "symbolic-ref", "--quiet", "--short", "HEAD") == branch
            and path_common_dir(replacement) == path_common_dir(root))


def archive_snapshot(snapshot: pathlib.Path, archive: pathlib.Path) -> dict[str, str]:
    if not snapshot.exists():
        return {}
    data = snapshot.read_bytes()
    archive.parent.mkdir(parents=True, exist_ok=True)
    if archive.exists():
        if archive.read_bytes() != data:
            raise MigrationError("historical snapshot archive differs from active snapshot")
    else:
        archive.write_bytes(data)
    return {"snapshot_path": str(archive), "snapshot_sha256": hashlib.sha256(data).hexdigest(),
            "snapshot_bytes_b64": base64.b64encode(data).decode("ascii")}


def finish_active_snapshot_cleanup(journal: dict[str, Any]) -> None:
    """Remove an old active snapshot only after its archived bytes are proven."""
    raw_source = str(journal.get("old_snapshot_path") or "")
    if not raw_source:
        return
    source = pathlib.Path(raw_source)
    if not source.exists():
        return
    archive = pathlib.Path(str(journal.get("historical_snapshot_path") or ""))
    if not archive.exists() or archive.read_bytes() != source.read_bytes():
        raise MigrationError("committed migration lacks a matching historical snapshot archive")
    source.unlink()


def readback_committed_migration(mapping_path: pathlib.Path, task_uid: str,
                                 receipt: dict[str, Any]) -> dict[str, Any]:
    """Reload and validate the durable migration record before authority is emitted."""
    try:
        committed_mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise MigrationError(f"cannot read committed task mapping: {exc}") from exc
    record = require_record(committed_mapping, task_uid)
    committed_receipt = record.get("branch_identity_migration_receipt")
    if not isinstance(committed_receipt, dict) or committed_receipt != receipt:
        raise MigrationError("committed task mapping receipt disagrees with migration receipt")
    if committed_receipt.get("digest") != digest(committed_receipt):
        raise MigrationError("committed migration receipt digest is invalid")

    migration = record.get("branch_identity_migration")
    historical = (record.get("historical_epochs") or {}).get(str(receipt.get("old_epoch")))
    invalidated = record.get("invalidated_authority")
    receipt_invalidation = receipt.get("invalidated_authority")
    if not isinstance(migration, dict) or not isinstance(historical, dict):
        raise MigrationError("committed task mapping lacks migration history")
    if not isinstance(invalidated, dict) or not isinstance(receipt_invalidation, dict):
        raise MigrationError("committed task mapping lacks invalidated authority")

    pairs = (
        ("bootstrap_epoch", "new_epoch"),
        ("canonical_worktree", "new_worktree"),
        ("task_branch", "new_branch"),
    )
    if any(record.get(record_field) != receipt.get(receipt_field)
           for record_field, receipt_field in pairs):
        raise MigrationError("committed task identity disagrees with migration receipt")
    migration_fields = (
        "old_epoch", "new_epoch", "old_worktree", "old_branch", "old_common_dir",
        "new_worktree", "new_branch", "new_common_dir", "implementation_head",
        "comparison_ref", "comparison_oid",
    )
    if any(migration.get(field) != receipt.get(field) for field in migration_fields):
        raise MigrationError("committed migration record disagrees with migration receipt")
    if migration.get("digest") != receipt.get("migration_record_sha256"):
        raise MigrationError("committed migration record digest disagrees with migration receipt")

    historical_record = historical.get("task_record")
    if not isinstance(historical_record, dict) or any((
            historical_record.get("task_uid") != task_uid,
            historical_record.get("bootstrap_epoch") != receipt.get("old_epoch"),
            historical_record.get("canonical_worktree") != receipt.get("old_worktree"),
            historical_record.get("task_branch") != receipt.get("old_branch"),
    )):
        raise MigrationError("committed historical task record disagrees with migration receipt")
    if historical.get("snapshot_sha256", "") != receipt.get("historical_artifact_digests", {}).get("snapshot_sha256", ""):
        raise MigrationError("committed historical snapshot disagrees with migration receipt")
    if (invalidated.get("migration_receipt_sha256") != receipt.get("digest") or
            invalidated.get("reason") != receipt_invalidation.get("reason") or
            invalidated.get("fields") != receipt_invalidation.get("fields")):
        raise MigrationError("committed invalidated authority disagrees with migration receipt")
    return record


def run(args: argparse.Namespace) -> dict[str, Any]:
    root = pathlib.Path(args.repo_root).resolve()
    mapping_path = pathlib.Path(args.tasks_json).resolve()
    replacement = pathlib.Path(args.replacement_worktree).resolve()
    if replacement == root:
        raise MigrationError("replacement worktree must differ from active worktree")
    branch_ref(args.replacement_branch)
    journal_file = journal_path(root, args.task_uid)

    with mapping_lock(mapping_path) as mapping:
        record = require_record(mapping, args.task_uid)
        old_epoch = positive_integer(record.get("bootstrap_epoch", 1), "bootstrap epoch")
        prior_journal = load_journal(journal_file, args.task_uid)
        if prior_journal.get("state") == "committed":
            receipt = prior_journal.get("receipt")
            if (isinstance(receipt, dict) and receipt.get("task_uid") == args.task_uid
                    and receipt.get("digest") == digest(receipt)
                    and record.get("bootstrap_epoch") == receipt.get("new_epoch")
                    and record.get("canonical_worktree") == receipt.get("new_worktree")
                    and record.get("task_branch") == receipt.get("new_branch")):
                finish_active_snapshot_cleanup(prior_journal)
                return receipt
            raise MigrationError("committed migration journal disagrees with active task mapping")
        recovered_receipt = record.get("branch_identity_migration_receipt")
        if (isinstance(recovered_receipt, dict) and recovered_receipt.get("task_uid") == args.task_uid
                and recovered_receipt.get("digest") == digest(recovered_receipt)
                and record.get("bootstrap_epoch") == recovered_receipt.get("new_epoch")
                and record.get("canonical_worktree") == recovered_receipt.get("new_worktree")
                and record.get("task_branch") == recovered_receipt.get("new_branch")):
            finish_active_snapshot_cleanup(prior_journal)
            update_journal(journal_file, {"task_uid": args.task_uid, "state": "committed",
                                          "receipt": recovered_receipt})
            return recovered_receipt
        if pathlib.Path(str(record["canonical_worktree"])).resolve() != root:
            raise MigrationError("active canonical worktree does not match --repo-root")
        if git(root, "symbolic-ref", "--quiet", "--short", "HEAD") != record["task_branch"]:
            raise MigrationError("active source branch identity disagrees with task mapping")
        for other_uid, other in (mapping.get("tasks") or {}).items():
            if other_uid != args.task_uid and isinstance(other, dict) and other.get("task_branch") == record["task_branch"]:
                raise MigrationError("active source branch identity belongs to another task")
        head = git(root, "rev-parse", "HEAD")
        comparison_oid = git(root, "rev-parse", "--verify", args.comparison_ref + "^{commit}")
        common_dir = path_common_dir(root)
        authoritative_repository, authoritative_remote_names = authoritative_remotes(
            root, str(record["repository"])
        )
        snapshot = root / ".pm" / "scratch" / args.task_uid / "bootstrap-task-snapshot.json"
        snapshot_bytes = snapshot.read_bytes() if snapshot.exists() else b""
        journal = update_journal(journal_file, {
            "schema": JOURNAL_SCHEMA, "task_uid": args.task_uid,
            "repository": record["repository"], "issue_number": record["issue_number"],
            "project_item_id": record["project_item_id"], "owner_role": record["owner_role"],
            "status": record["status"], "old_epoch": old_epoch, "old_worktree": str(root),
            "old_branch": record["task_branch"], "old_common_dir": str(common_dir),
            "old_snapshot_path": str(snapshot) if snapshot.exists() else "",
            "old_snapshot_sha256": hashlib.sha256(snapshot_bytes).hexdigest() if snapshot_bytes else "",
            "replacement_worktree": str(replacement),
            "replacement_branch": args.replacement_branch, "implementation_head": head,
            "comparison_ref": args.comparison_ref, "comparison_oid": comparison_oid,
            "authoritative_repository": authoritative_repository,
            "authoritative_remote_names": authoritative_remote_names,
            "resume_command": " ".join([sys.executable, str(pathlib.Path(__file__).resolve()),
                "--repo-root", str(root), "--task-uid", args.task_uid]),
        })
        if not journal.get("state"):
            journal = update_journal(journal_file, {"task_uid": args.task_uid, "state": "intent"})
        local_exists = git_ok(root, "show-ref", "--verify", "--quiet", branch_ref(args.replacement_branch))
        tracking_refs = git(root, "for-each-ref", "--format=%(refname)", "refs/remotes").splitlines()
        tracking_remote_exists = any(
            name == f"refs/remotes/{remote}/{args.replacement_branch}"
            for remote in authoritative_remote_names for name in tracking_refs
        )
        try:
            live_remote_exists, remote_collisions = live_remote_branch_collision(
                root, args.replacement_branch, authoritative_remote_names
            )
        except MigrationError as exc:
            update_journal(journal_file, {"task_uid": args.task_uid, "state": "capability_blocked",
                                          "collision": {"remote_branch": False,
                                                        "remote_query_uncertain": True}})
            raise
        remote_exists = tracking_remote_exists or live_remote_exists
        proven = local_exists and replacement_is_proven(root, replacement, args.replacement_branch, head, journal)
        if (local_exists or remote_exists or replacement.exists()) and not proven:
            update_journal(journal_file, {"task_uid": args.task_uid, "state": "capability_blocked",
                                          "collision": {"local_branch": local_exists, "remote_branch": remote_exists,
                                                        "live_remote_branch": live_remote_exists,
                                                        "remote_names": remote_collisions,
                                                        "replacement_exists": replacement.exists()}})
            if live_remote_exists:
                raise MigrationError("remote branch uniqueness precondition failed")
            raise MigrationError("branch uniqueness precondition failed")
        if not proven:
            if replacement.exists():
                raise MigrationError("replacement worktree path already exists")
            git(root, "worktree", "add", "-b", args.replacement_branch, str(replacement), head)
            update_journal(journal_file, {"task_uid": args.task_uid, "state": "replacement_branch_created"})
            if os.environ.get("OASIS7_PM_TEST_MIGRATION_CRASH_AFTER") == "replacement_branch_created":
                raise MigrationError("injected crash after replacement branch created")

        moved_comparison_oid = os.environ.get(
            "OASIS7_PM_TEST_MIGRATION_COMPARISON_OID_AFTER_BRANCH_CREATED"
        )
        if moved_comparison_oid:
            git(root, "update-ref", args.comparison_ref, moved_comparison_oid)

        if (git(replacement, "rev-parse", "HEAD") != head or
                git(replacement, "symbolic-ref", "--quiet", "--short", "HEAD") != args.replacement_branch or
                git(root, "rev-parse", branch_ref(args.replacement_branch)) != head or
                path_common_dir(replacement) != common_dir):
            raise MigrationError("replacement identity readback failed")
        if git(replacement, "rev-parse", "--verify", args.comparison_ref + "^{commit}") != comparison_oid:
            raise MigrationError("comparison ref OID changed after replacement branch creation")

        archive = root / ".pm" / "scratch" / args.task_uid / "historical-epochs" / str(old_epoch) / "bootstrap-task-snapshot.json"
        historical = {"task_record": json.loads(json.dumps(record)), **archive_snapshot(snapshot, archive)}
        update_journal(journal_file, {"task_uid": args.task_uid,
                                      "historical_snapshot_path": historical.get("snapshot_path", "")})
        if os.environ.get("OASIS7_PM_TEST_MIGRATION_CRASH_AFTER") == "historical_snapshot_archived":
            raise MigrationError("injected crash after historical snapshot archived")
        new_epoch = old_epoch + 1
        migrated_at = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
        migration_record = {
            "issuer": args.issuer, "reason": args.reason, "migrated_at": migrated_at,
            "old_epoch": old_epoch, "new_epoch": new_epoch, "old_worktree": str(root),
            "old_branch": record["task_branch"], "old_common_dir": str(common_dir),
            "new_worktree": str(replacement), "new_branch": args.replacement_branch,
            "new_common_dir": str(path_common_dir(replacement)), "implementation_head": head,
            "comparison_ref": args.comparison_ref, "comparison_oid": comparison_oid,
        }
        migration_record["digest"] = digest(migration_record)
        invalidated_fields = ["bootstrap_snapshot", "phase_receipts", "phase_receipt_sha256", "evidence",
                              "claim_verifications", "pr_number", "pr_url", "merge_receipt",
                              "merge_receipt_sha256", "main_sync_receipt", "cleanup_receipt"]
        receipt_invalidation = {"reason": args.reason, "fields": invalidated_fields}
        receipt = {
            "schema": "oasis7_task_branch_identity_migration_receipt/v1", "task_uid": args.task_uid,
            "repository": authoritative_repository,
            "old_epoch": old_epoch, "new_epoch": new_epoch, "implementation_head": head,
            "comparison_ref": args.comparison_ref, "comparison_oid": comparison_oid,
            "old_worktree": str(root), "old_branch": record["task_branch"], "old_common_dir": str(common_dir),
            "new_worktree": str(replacement), "new_branch": args.replacement_branch,
            "new_common_dir": str(path_common_dir(replacement)),
            "journal_revision": int(journal.get("revision", 1)),
            "historical_artifact_digests": {"snapshot_sha256": historical.get("snapshot_sha256", "")},
            "migration_record_sha256": migration_record["digest"],
            "invalidated_authority": receipt_invalidation,
        }
        receipt["digest"] = digest(receipt)
        new_record = json.loads(json.dumps(record))
        new_record.update({"canonical_worktree": str(replacement), "task_branch": args.replacement_branch,
                           "bootstrap_epoch": new_epoch, "workflow_phase": "bootstrap",
                           "workflow_state": "action_required", "phase_receipts": {}, "evidence": {},
                           "branch_identity_migration": migration_record,
                           "branch_identity_migration_receipt": receipt,
                           "historical_epochs": dict(record.get("historical_epochs") or {}),
                           "invalidated_authority": {"migration_receipt_sha256": receipt["digest"],
                                                     "reason": args.reason, "fields": invalidated_fields}})
        new_record["historical_epochs"][str(old_epoch)] = historical
        for field in invalidated_fields:
            if field not in {"bootstrap_snapshot", "phase_receipts", "evidence"}:
                new_record.pop(field, None)
        mapping["tasks"][args.task_uid] = new_record
        durable_store.atomic_replace_json(mapping_path, mapping)
        readback_committed_migration(mapping_path, args.task_uid, receipt)
        if os.environ.get("OASIS7_PM_TEST_MIGRATION_CRASH_AFTER") == "mapping_committed":
            raise MigrationError("injected crash after mapping committed")

    # The old immutable file is retained in the historical ledger before it is
    # removed from the active path, allowing the normal bootstrap command to
    # create a new snapshot for the new epoch.
    if snapshot.exists():
        snapshot.unlink()
    update_journal(journal_file, {"task_uid": args.task_uid, "state": "committed", "receipt": receipt})
    return receipt


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--repo-root", required=True)
    result.add_argument("--task-uid", required=True)
    result.add_argument("--tasks-json", required=True)
    result.add_argument("--replacement-worktree", required=True)
    result.add_argument("--replacement-branch", required=True)
    result.add_argument("--comparison-ref", required=True)
    result.add_argument("--issuer", required=True)
    result.add_argument("--reason", required=True)
    result.add_argument("--json", action="store_true")
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        receipt = run(args)
    except MigrationError as exc:
        print(f"migrate-task-branch-identity: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, sort_keys=True) if args.json else receipt["digest"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
