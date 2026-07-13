#!/usr/bin/env python3
"""Fail-closed production checkpoint owner for the TPM workflow.

The supervisor owns checkpoint creation and identity.  It does not infer a
successful bootstrap from local files and it never advances to routing until a
fixed, repo-integrated bootstrap producer can supply independently verifiable
GitHub Project, owner, mapping, and canonical-worktree evidence.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import subprocess
from pathlib import Path
from typing import NoReturn
from uuid import uuid4


SCHEMA = "tpm-production-supervisor/v2"
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
# Durable outcome used once a trusted bootstrap reaches professional dispatch.
# It is capability_blocked (not external_wait) until an immutable collaboration
# runtime can provide the dispatch attestation.
DISPATCH_CAPABILITY_BLOCK = {
    "status": "capability_blocked",
    "blocker": {"class": "dispatch_attestation_unavailable"},
}


def emit(value: dict, code: int = 0) -> NoReturn:
    print(json.dumps(value, sort_keys=True))
    raise SystemExit(code)


def blocked(blocker_class: str, *, phase: str | None = None,
            resume_condition: str | None = None, **extra: object) -> dict:
    value: dict[str, object] = {
        "status": "capability_blocked",
        "capability_status": "blocked",
        "production_passed": False,
        "automatic": False,
        "blocker": {"class": blocker_class},
    }
    if phase is not None:
        value["phase"] = phase
    if resume_condition is not None:
        value["blocker"]["resume_condition"] = resume_condition  # type: ignore[index]
    value.update(extra)
    return value


def git(repo: Path, *args: str) -> str | None:
    proc = subprocess.run(
        ["git", "-C", str(repo), *args], text=True, capture_output=True
    )
    return proc.stdout.strip() if proc.returncode == 0 else None


def canonical_state(repo: Path, task_uid: str) -> Path:
    return (repo / ".pm" / "tasks" / f"{task_uid}.workflow.json").resolve()


def create_exclusive(path: Path, value: dict) -> bool:
    """Create the first checkpoint exactly once; never replace prior truth."""
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        return False
    try:
        payload = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
        with os.fdopen(fd, "wb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
    except BaseException:
        path.unlink(missing_ok=True)
        raise
    return True


def canonical_identity(repo: Path, task_uid: str, state_path: Path) -> dict:
    worktree = git(repo, "rev-parse", "--show-toplevel")
    task_branch = git(repo, "symbolic-ref", "--quiet", "--short", "HEAD")
    repository = git(repo, "remote", "get-url", "origin") or str(repo)
    default_branch = git(repo, "symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD")
    if default_branch and "/" in default_branch:
        default_branch = default_branch.split("/", 1)[1]
    default_branch = default_branch or "main"
    return {
        "repository": repository,
        "canonical_worktree": worktree,
        "task_uid": task_uid,
        "task_branch": task_branch,
        "default_branch": default_branch,
        "checkpoint": str(state_path),
        # These authorities must later be filled by trusted live producers.
        "merge_receipt": None,
        "main_sync_authority": None,
        "cleanup_authority": None,
    }


def checkpoint_for(repo: Path, task_uid: str, state_path: Path) -> dict:
    now = dt.datetime.now(dt.timezone.utc).isoformat()
    identity = canonical_identity(repo, task_uid, state_path)
    return {
        "schema": SCHEMA,
        "revision": 1,
        "lease_token": str(uuid4()),
        "started_at": now,
        "updated_at": now,
        "phase": "bootstrap",
        "status": "capability_blocked",
        "capability_status": "blocked",
        "production_passed": False,
        "automatic": False,
        "task_uid": task_uid,
        "repository": identity["repository"],
        "repo": str(repo),
        "state": str(state_path),
        "completed": [],
        "bootstrap_authority": {
            "github_project_task_truth": None,
            "owner_role": None,
            "task_mapping": None,
            "canonical_worktree": identity["canonical_worktree"],
            "trusted_bootstrap_receipt": None,
        },
        "terminal_authority": identity,
        "wake_owner": {"installed": False},
        "blocker": {
            "class": "bootstrap_attestation_unavailable",
            "resume_condition": (
                "a fixed repo-integrated producer must attest GitHub Project task truth, "
                "owner role, task mapping, and canonical worktree"
            ),
        },
    }


parser = argparse.ArgumentParser()
parser.add_argument("--initialize", action="store_true")
parser.add_argument("--resume", action="store_true")
parser.add_argument("--task-uid")
parser.add_argument("--repo", type=Path)
parser.add_argument("--state", type=Path)
parser.add_argument("--run-to-completion", action="store_true")
parser.add_argument("--json", action="store_true")
args, unknown = parser.parse_known_args()

if unknown:
    emit(blocked("unsupported_production_operation"), 75)
if args.initialize == args.resume:
    emit({"error": "exactly_one_of_initialize_or_resume_required"}, 64)
if args.resume and not all((args.task_uid, args.repo)):
    # Legacy or caller-authored state cannot mint the missing canonical task and
    # repository identity.  Reject it without parsing or mutating its bytes.
    emit(blocked(
        "terminal_slice_authority_untrusted",
        resume_condition="resume requires canonical task_uid and repo identity",
    ), 75)
if not all((args.task_uid, args.repo, args.state, args.run_to_completion)):
    emit({"error": "task_uid_repo_state_and_run_to_completion_required"}, 64)
if not TASK_UID_RE.fullmatch(args.task_uid):
    emit(blocked("invalid_task_uid", phase="bootstrap"), 75)

repo = args.repo.resolve()
state_path = args.state.resolve()

# Existing state is immutable to --initialize, regardless of whether a caller
# also supplied an invalid path.  This makes accidental re-initialization
# observably harmless and preserves the prior bytes exactly.
if args.initialize and state_path.exists():
    emit(blocked("checkpoint_already_exists", phase="bootstrap"), 75)

expected_state = canonical_state(repo, args.task_uid)
if state_path != expected_state:
    emit(blocked("noncanonical_checkpoint_path", phase="bootstrap"), 75)

repo_root = git(repo, "rev-parse", "--show-toplevel")
if repo_root is None or Path(repo_root).resolve() != repo:
    emit(blocked("canonical_worktree_readback_failed", phase="bootstrap"), 75)

if args.resume:
    if not state_path.is_file():
        emit(blocked("checkpoint_missing", phase="bootstrap"), 75)
    try:
        prior = json.loads(state_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        emit(blocked("checkpoint_invalid", phase="bootstrap"), 75)
    terminal = prior.get("terminal_authority", {})
    if (
        prior.get("schema") != SCHEMA
        or prior.get("task_uid") != args.task_uid
        or Path(prior.get("repo", "")).resolve() != repo
        or terminal.get("checkpoint") != str(state_path)
    ):
        emit(blocked("checkpoint_identity_mismatch", phase="bootstrap"), 75)
    # Resume cannot advance until a trusted runtime connector validates and
    # consumes the bootstrap receipt using compare-and-swap on revision/lease.
    emit(blocked(
        "production_resume_connector_unavailable",
        phase=prior.get("phase", "bootstrap"),
        resume_condition="install the fixed bootstrap validator and checkpoint CAS consumer",
        wake_owner={"installed": False},
    ), 75)

state = checkpoint_for(repo, args.task_uid, state_path)
if not create_exclusive(state_path, state):
    emit(blocked("checkpoint_already_exists", phase="bootstrap"), 75)
emit(state, 75)
