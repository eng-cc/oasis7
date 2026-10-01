#!/usr/bin/env python3
"""Disposable-Git boundary tests for the v2 resource cleanup executor."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
import traceback
import uuid


SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))
SPEC = importlib.util.spec_from_file_location(
    "resource_cleanup_executor", SCRIPT_DIR / "resource-cleanup-executor.py"
)
assert SPEC and SPEC.loader
EXECUTOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXECUTOR)


def git(cwd: pathlib.Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(cwd), *args], text=True, capture_output=True
    )
    if result.returncode:
        raise AssertionError(
            f"git {args!r} failed ({result.returncode}): {result.stderr.strip()}"
        )
    return result.stdout.strip()


class Fixture:
    def __init__(self, base: pathlib.Path):
        self.root = base
        self.repo = base / "default"
        self.remote = base / "origin.git"
        self.worktree = base / "task-worktree"
        self.uid = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        self.repository = "eng-cc/oasis7"
        self.branch = "task/cleanup-safety-fixture"

        subprocess.run(["git", "init", "--bare", "-q", str(self.remote)], check=True)
        self.repo.mkdir()
        git(self.repo, "init", "-q", "-b", "main")
        git(self.repo, "config", "user.email", "qa@example.invalid")
        git(self.repo, "config", "user.name", "Disposable QA")
        (self.repo / ".gitignore").write_text("target\n*.private\n", encoding="utf-8")
        (self.repo / "tracked.txt").write_text("fixture\n", encoding="utf-8")
        git(self.repo, "add", ".gitignore", "tracked.txt")
        git(self.repo, "commit", "-qm", "fixture base")
        git(self.repo, "remote", "add", "origin", str(self.remote))
        git(self.repo, "push", "-q", "origin", "main")
        git(self.repo, "worktree", "add", "-qb", self.branch, str(self.worktree), "main")
        git(self.repo, "push", "-q", "origin", self.branch)

        common_dir = pathlib.Path(git(self.repo, "rev-parse", "--git-common-dir"))
        if not common_dir.is_absolute():
            common_dir = self.repo / common_dir
        common_dir = common_dir.resolve()
        admin_dir = pathlib.Path(git(self.worktree, "rev-parse", "--absolute-git-dir")).resolve()
        self.instance_id = str(uuid.uuid4())
        (admin_dir / "oasis7-instance-id").write_text(self.instance_id + "\n", encoding="utf-8")

        mapping_path = self.repo / ".pm/github-project-sync/tasks.json"
        mapping_path.parent.mkdir(parents=True)
        mapping_path.write_text(json.dumps({"version": 1, "tasks": {self.uid: {
            "task_uid": self.uid,
            "status": "done",
            "repository": self.repository,
            "canonical_worktree": str(self.worktree),
            "task_branch": self.branch,
            "default_branch": "main",
            "worktree_registration": {
                "common_dir": str(common_dir),
                "admin_dir": str(admin_dir),
                "instance_id": self.instance_id,
            },
        }}}) + "\n", encoding="utf-8")

        self.receipt_root = EXECUTOR._receipt_root(self.repo, self.uid, create=True)
        (self.receipt_root / "merge-receipt.json").write_text("{}\n", encoding="utf-8")
        receipt = {
            "receipt_type": "oasis7_terminal_delivery",
            "schema_version": 2,
            "task_uid": self.uid,
            "repository": self.repository,
            "completion_semantics": "delivery_only",
            "head_oid": git(self.worktree, "rev-parse", "HEAD"),
            "worktree": str(self.worktree),
            "branch": self.branch,
        }
        (self.receipt_root / "terminal-delivery-receipt.json").write_text(
            json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8"
        )

    def execute(self, *, preflight: bool = False,
                overrides: dict[str, object] | None = None) -> tuple[int, dict]:
        """Run the real CLI parser and cleanup path with only live delivery readback isolated."""
        hooks = {"_run_delivery_preflight": lambda _repo, _uid: {
                     "delivery": {"state": "complete", "protocol_version": 2}
                 },
                 # The cleanup remote is a local bare repo. Pin its canonical
                 # GitHub identity in this isolated fixture so no network lookup is possible.
                 "_origin_repository": lambda _repo: self.repository}
        hooks.update(overrides or {})
        previous = {name: getattr(EXECUTOR, name) for name in hooks}
        old_argv = sys.argv
        for name, function in hooks.items():
            setattr(EXECUTOR, name, function)
        sys.argv = [str(SCRIPT_DIR / "resource-cleanup-executor.py"),
                    "--repo-root", str(self.repo), "--task-uid", self.uid,
                    "--delivery", "--json"]
        if preflight:
            sys.argv.append("--preflight")
        stdout = io.StringIO()
        try:
            with contextlib.redirect_stdout(stdout):
                code = EXECUTOR.main()
        finally:
            for name, function in previous.items():
                setattr(EXECUTOR, name, function)
            sys.argv = old_argv
        return code, json.loads(stdout.getvalue())


def test_preflight_is_read_only(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    code, payload = fixture.execute(preflight=True)
    assert code == 0, payload
    assert payload["status"] == "ready", payload
    assert payload["cleanup"]["local_branch"]["state"] == "ready", payload
    assert fixture.worktree.is_dir()
    assert git(fixture.repo, "show-ref", "--verify", f"refs/heads/{fixture.branch}")
    assert git(fixture.remote, "show-ref", "--verify", f"refs/heads/{fixture.branch}")
    assert not (fixture.receipt_root / "resource-cleanup.json").exists()
    assert not (fixture.receipt_root / "resource-cleanup-journal.jsonl").exists()


def test_exact_registered_resources_are_removed_and_journal_linked(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    code, payload = fixture.execute()
    assert code == 0, payload
    assert payload["status"] == "cleaned", payload
    assert not fixture.worktree.exists()
    assert not any(row["path"] == str(fixture.worktree.resolve())
                   for row in EXECUTOR._worktree_rows(fixture.repo))
    local = subprocess.run(["git", "-C", str(fixture.repo), "show-ref", "--verify", "--quiet",
                            f"refs/heads/{fixture.branch}"])
    remote = subprocess.run(["git", "-C", str(fixture.remote), "show-ref", "--verify", "--quiet",
                             f"refs/heads/{fixture.branch}"])
    assert local.returncode == 1, "local branch still exists after cleanup"
    assert remote.returncode == 1, "remote branch still exists after cleanup"
    assert {row["state"] for row in payload["resources"]} == {"removed"}, payload

    for resource in payload["resources"]:
        row = EXECUTOR.read_cleanup_record(fixture.receipt_root, fixture.uid,
                                           fixture.repository, resource["kind"], resource["resource_id"])
        assert row["state"] == "removed", row


def test_untracked_tracked_and_ignored_evidence_are_retained(base: pathlib.Path) -> None:
    cases = (
        ("untracked", lambda worktree: (worktree / "manual-results.json").write_text("keep\n"),
         "tracked_or_untracked_content", "manual-results.json"),
        ("tracked", lambda worktree: (worktree / "tracked.txt").write_text("edited evidence\n"),
         "tracked_or_untracked_content", "tracked.txt"),
        ("ignored", lambda worktree: (worktree / "draft.private").write_text("private evidence\n"),
         "protected_or_unknown_ignored_content", "draft.private"),
    )
    for name, create_content, reason, filename in cases:
        case = base / name
        case.mkdir()
        fixture = Fixture(case)
        expected = "edited evidence\n" if name == "tracked" else (
            "private evidence\n" if name == "ignored" else "keep\n")
        create_content(fixture.worktree)
        code, payload = fixture.execute()
        assert code == 3 and payload["status"] == "cleanup_deferred", payload
        worktree = next(row for row in payload["resources"] if row["kind"] == "worktree")
        assert worktree["state"] == "retained" and worktree["reason"] == reason, worktree
        assert (fixture.worktree / filename).read_text(encoding="utf-8") == expected
        assert git(fixture.repo, "show-ref", "--verify", f"refs/heads/{fixture.branch}")
        remote = subprocess.run(["git", "-C", str(fixture.remote), "show-ref", "--verify", "--quiet",
                                 f"refs/heads/{fixture.branch}"])
        assert remote.returncode == 1, "remote ref outcome should be recorded independently"


def test_locked_and_active_worktrees_are_retained(base: pathlib.Path) -> None:
    locked_case = base / "locked"
    locked_case.mkdir()
    locked = Fixture(locked_case)
    git(locked.repo, "worktree", "lock", "--reason", "QA retained lock", str(locked.worktree))
    code, payload = locked.execute()
    assert code == 3 and payload["status"] == "cleanup_deferred", payload
    locked_row = next(row for row in payload["resources"] if row["kind"] == "worktree")
    assert locked_row["state"] == "retained" and locked_row["reason"] == "worktree_locked", locked_row
    assert locked.worktree.is_dir()

    active_case = base / "active"
    active_case.mkdir()
    active = Fixture(active_case)
    process = subprocess.Popen([
        sys.executable, "-c", "import sys,time; time.sleep(30)", str(active.worktree)
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(0.15)
        code, payload = active.execute()
    finally:
        process.terminate()
        process.wait(timeout=5)
    assert code == 3 and payload["status"] == "cleanup_deferred", payload
    active_row = next(row for row in payload["resources"] if row["kind"] == "worktree")
    assert active_row["state"] == "retained" and active_row["reason"] == "active_process", active_row
    assert active.worktree.is_dir()


def test_only_external_target_cache_is_discarded(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    cache = base / "external-cache"
    cache.mkdir()
    protected = cache / "cache-sentinel.bin"
    protected.write_bytes(b"cache data outside worktree\x00\xff")
    (fixture.worktree / "target").symlink_to(cache, target_is_directory=True)
    code, payload = fixture.execute()
    assert code == 0 and payload["status"] == "cleaned", payload
    assert not fixture.worktree.exists()
    assert protected.read_bytes() == b"cache data outside worktree\x00\xff"


def test_same_path_and_oid_recreation_is_retained(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    first_code, first_payload = fixture.execute()
    assert first_code == 0 and first_payload["status"] == "cleaned", first_payload
    expected_oid = json.loads((fixture.receipt_root / "terminal-delivery-receipt.json").read_text())["head_oid"]
    git(fixture.repo, "branch", fixture.branch, expected_oid)
    git(fixture.remote, "update-ref", f"refs/heads/{fixture.branch}", expected_oid)
    git(fixture.repo, "worktree", "add", str(fixture.worktree), fixture.branch)
    new_admin = pathlib.Path(git(fixture.worktree, "rev-parse", "--absolute-git-dir")).resolve()
    new_instance = str(uuid.uuid4())
    (new_admin / "oasis7-instance-id").write_text(new_instance + "\n", encoding="utf-8")

    code, payload = fixture.execute()
    assert code == 3 and payload["status"] == "cleanup_deferred", payload
    row = next(row for row in payload["resources"] if row["kind"] == "worktree")
    assert row["state"] == "retained" and row["reason"] == "path_reused_after_release_or_uncertain_cleanup", row
    assert fixture.worktree.is_dir()
    assert git(fixture.worktree, "rev-parse", "HEAD") == expected_oid
    assert (new_admin / "oasis7-instance-id").read_text().strip() == new_instance
    assert git(fixture.repo, "rev-parse", f"refs/heads/{fixture.branch}") == expected_oid
    assert git(fixture.remote, "rev-parse", f"refs/heads/{fixture.branch}") == expected_oid


def test_local_cas_and_remote_lease_preserve_replaced_oids(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    original = git(fixture.worktree, "rev-parse", "HEAD")
    git(fixture.repo, "switch", "main")
    (fixture.repo / "race-one.txt").write_text("new local ref tip\n", encoding="utf-8")
    git(fixture.repo, "add", "race-one.txt")
    git(fixture.repo, "commit", "-qm", "race one")
    local_replacement = git(fixture.repo, "rev-parse", "HEAD")
    (fixture.repo / "race-two.txt").write_text("new remote ref tip\n", encoding="utf-8")
    git(fixture.repo, "add", "race-two.txt")
    git(fixture.repo, "commit", "-qm", "race two")
    remote_replacement = git(fixture.repo, "rev-parse", "HEAD")
    git(fixture.repo, "push", "-q", "origin", "main")
    seen: set[str] = set()
    real_run = EXECUTOR._run

    def race(args: list[str], *, cwd: pathlib.Path | None = None, check: bool = True):
        if args[:2] == ["git", "-C"] and "update-ref" in args and "-d" in args and "local" not in seen:
            subprocess.run(["git", "-C", str(fixture.repo), "update-ref",
                            f"refs/heads/{fixture.branch}", local_replacement], check=True)
            seen.add("local")
        if args[:2] == ["git", "-C"] and "push" in args and any(
                value.startswith("--force-with-lease=") for value in args) and "remote" not in seen:
            subprocess.run(["git", "-C", str(fixture.remote), "update-ref",
                            f"refs/heads/{fixture.branch}", remote_replacement], check=True)
            seen.add("remote")
        return real_run(args, cwd=cwd, check=check)

    code, payload = fixture.execute(overrides={"_run": race})
    assert code == 3 and payload["status"] == "cleanup_deferred", payload
    assert seen == {"local", "remote"}, seen
    local_row = next(row for row in payload["resources"] if row["kind"] == "local_branch")
    remote_row = next(row for row in payload["resources"] if row["kind"] == "remote_branch")
    assert local_row["state"] == "retained", local_row
    assert remote_row["state"] == "retained", remote_row
    assert git(fixture.repo, "rev-parse", f"refs/heads/{fixture.branch}") == local_replacement
    assert git(fixture.remote, "rev-parse", f"refs/heads/{fixture.branch}") == remote_replacement
    assert original != local_replacement != remote_replacement


def test_crash_after_remove_reconciles_recreated_same_oid_instance(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    real_state = EXECUTOR._worktree_state

    def crash_after_remove(*args, **kwargs):
        outcome = real_state(*args, **kwargs)
        if kwargs.get("mutate") and outcome.get("state") == "removed":
            raise SystemExit(97)
        return outcome

    try:
        fixture.execute(overrides={"_worktree_state": crash_after_remove})
    except SystemExit as exc:
        assert exc.code == 97
    else:
        raise AssertionError("fault injection did not interrupt cleanup after worktree removal")
    journal = fixture.receipt_root / "resource-cleanup-journal.jsonl"
    events = [json.loads(line) for line in journal.read_text(encoding="utf-8").splitlines()]
    assert len(events) == 1 and events[0]["event"] == "intent", events
    assert not (fixture.receipt_root / "resource-cleanup.json").exists()

    expected_oid = git(fixture.repo, "rev-parse", f"refs/heads/{fixture.branch}")
    git(fixture.repo, "worktree", "add", str(fixture.worktree), fixture.branch)
    new_admin = pathlib.Path(git(fixture.worktree, "rev-parse", "--absolute-git-dir")).resolve()
    new_instance = str(uuid.uuid4())
    (new_admin / "oasis7-instance-id").write_text(new_instance + "\n", encoding="utf-8")

    code, payload = fixture.execute()
    assert code == 3 and payload["status"] == "cleanup_deferred", payload
    worktree_row = next(row for row in payload["resources"] if row["kind"] == "worktree")
    assert worktree_row["state"] == "retained" and worktree_row["reason"] == "cleanup_operation_uncertain", worktree_row
    assert fixture.worktree.is_dir() and git(fixture.worktree, "rev-parse", "HEAD") == expected_oid
    assert (new_admin / "oasis7-instance-id").read_text().strip() == new_instance
    assert json.loads((fixture.receipt_root / "resource-cleanup.json").read_text())["revision"] == 2


def test_snapshot_tampering_breaks_api_link_and_blocks_retry(base: pathlib.Path) -> None:
    fixture = Fixture(base)
    code, payload = fixture.execute()
    assert code == 0 and payload["status"] == "cleaned", payload
    snapshot_path = fixture.receipt_root / "resource-cleanup.json"
    original = snapshot_path.read_bytes()
    snapshot_path.write_bytes(original + b" ")
    resource = next(row for row in payload["resources"] if row["kind"] == "worktree")
    try:
        EXECUTOR.read_cleanup_record(fixture.receipt_root, fixture.uid,
                                     fixture.repository, "worktree", resource["resource_id"])
    except ValueError:
        pass
    else:
        raise AssertionError("public cleanup reader accepted snapshot bytes absent from the journal")
    code, retry = fixture.execute()
    assert code == 1 and retry["status"] == "blocked", retry
    assert "journal" in " ".join(retry["cleanup_blockers"]).lower()
    assert not fixture.worktree.exists()


def main() -> int:
    tests = (
        test_preflight_is_read_only,
        test_exact_registered_resources_are_removed_and_journal_linked,
        test_untracked_tracked_and_ignored_evidence_are_retained,
        test_locked_and_active_worktrees_are_retained,
        test_only_external_target_cache_is_discarded,
        test_same_path_and_oid_recreation_is_retained,
        test_local_cas_and_remote_lease_preserve_replaced_oids,
        test_crash_after_remove_reconciles_recreated_same_oid_instance,
        test_snapshot_tampering_breaks_api_link_and_blocks_retry,
    )
    with tempfile.TemporaryDirectory(prefix="oasis7-cleanup-qa-") as raw:
        base = pathlib.Path(raw).resolve()
        failures = 0
        for test in tests:
            case = base / test.__name__
            case.mkdir()
            try:
                test(case)
                print(f"PASS {test.__name__}")
            except Exception:
                failures += 1
                print(f"FAIL {test.__name__}")
                traceback.print_exc()
    passed = len(tests) - failures
    print(f"resource-cleanup-safety: {passed} passed, {failures} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
