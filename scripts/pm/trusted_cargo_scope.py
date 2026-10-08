#!/usr/bin/env python3
"""Execute the complete Cargo checker authority from the unique source merge base."""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tarfile
import tempfile
import io
import json
import os

POLICY_MAINTENANCE_PATHS = frozenset({
    "doc/engineering/workflow/source-of-truth.md",
    "doc/.governance/document-corpus/objects/48/4840d720cacf3f7d531e8a361fc146277494b75857c9bd904bbc4b0f700c6f41.json",
    "scripts/pm/check-cargo-package-scope", "scripts/pm/check-cargo-package-scope.test.py",
    ".pm/cargo-package-auxiliary-files.json", "scripts/ci-tests.sh",
    "scripts/ci-required-scope.v2.json", "scripts/ci-required-scope-audit-contract.test.sh",
    ".github/workflows/rust.yml",
})


def extract_scripts(repo: Path, revision: str, destination: Path) -> None:
    archive = subprocess.check_output(["git", "-C", str(repo), "archive", revision, "scripts"])
    with tarfile.open(fileobj=io.BytesIO(archive)) as contents:
        for member in contents.getmembers():
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or member.issym() or member.islnk():
                raise ValueError("unsafe trusted Cargo script archive")
        contents.extractall(destination)


def changed_paths(repo: Path, base: str, head: str) -> list[str]:
    raw = subprocess.check_output(["git", "-C", str(repo), "diff", "--name-status", "--find-renames", "-z", base, head])
    fields = raw.split(b"\0")
    if fields.pop() != b"":
        raise ValueError("incomplete Git changed-path records")
    paths = []
    while fields:
        status = fields.pop(0).decode("ascii")
        count = 2 if status.startswith(("R", "C")) else 1
        if len(fields) < count:
            raise ValueError("incomplete Git rename endpoints")
        for _ in range(count):
            path = fields.pop(0).decode("utf-8")
            if any(delimiter in path for delimiter in (";", "\n", "\r")):
                raise ValueError("changed path cannot be represented in planner metadata")
            paths.append(path)
    return paths


def verify_full_plan(repo: Path, target: str, candidate: str, source: str, plan_file: str, execution_environment: dict[str, str] | None = None) -> bool:
    """Recompute B's complete coverage; bind the runner's actual selectors."""
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate full-plan key")
            result[key] = value
        return result
    plan = json.loads(Path(plan_file).read_text(), object_pairs_hook=unique_object)
    if not isinstance(plan, dict) or plan.get("scope") != "full":
        raise ValueError("trusted full-plan scope is not full")
    for key, expected in (("integration_base", target), ("source_head", candidate), ("source_scope_base", source)):
        if plan.get(key) != expected:
            raise ValueError("trusted full-plan identity mismatch: " + key)
    source_paths = changed_paths(repo, source, candidate)
    integration_paths = changed_paths(repo, target, candidate)
    paths = plan.get("changed_paths", "").split(";")
    if paths not in (source_paths, integration_paths):
        raise ValueError("trusted full-plan changed paths mismatch")
    with tempfile.TemporaryDirectory(prefix="trusted-cargo-planner-") as directory:
        authority = Path(directory)
        extract_scripts(repo, target, authority)
        command = [sys.executable, "-I", str(authority / "scripts/plan-rust-required-scope.py"),
                   "--event-name", "pull_request"]
        for path in paths:
            command.extend(("--changed-path", path))
        result = subprocess.run(command, cwd=repo, text=True, capture_output=True)
        if result.returncode:
            raise ValueError("trusted B full planner failed: " + result.stderr.strip())
        oracle = dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)
        natural_full = oracle.get("scope") == "full"
        if not natural_full:
            # Existing forced-full execution still uses ordinary S authority.
            # Its coverage is checked, but cannot grant either exception.
            command.extend(("--run-mode", "full_escalation"))
            result = subprocess.run(command, cwd=repo, text=True, capture_output=True)
            if result.returncode:
                raise ValueError("trusted B full coverage planner failed: " + result.stderr.strip())
            oracle = dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)
    # Config identity locates immutable emitting code; it never grants coverage
    # or full-plan exception authority. Equal config conservatively selects B.
    emitting = oracle
    if plan.get("planner_config_sha256") != oracle.get("planner_config_sha256"):
        config_bytes = subprocess.check_output(["git", "-C", str(repo), "show", candidate + ":scripts/ci-required-scope.v2.json"])
        if plan.get("planner_config_sha256") != "sha256:" + hashlib.sha256(config_bytes).hexdigest():
            raise ValueError("trusted full-plan emitting config identity mismatch")
        if plan.get("execution_contract") not in {"required-domain-split/v1", "required-domain-split/v2"}:
            raise ValueError("trusted full-plan emitting execution contract unsupported")
        if oracle.get("execution_contract") == "required-domain-split/v2" and plan.get("execution_contract") != "required-domain-split/v2":
            raise ValueError("trusted full-plan execution contract downgrade")
        with tempfile.TemporaryDirectory(prefix="trusted-cargo-emitting-planner-") as directory:
            authority = Path(directory)
            extract_scripts(repo, candidate, authority)
            command = [sys.executable, "-I", str(authority / "scripts/plan-rust-required-scope.py"), "--event-name", "pull_request"]
            for path in paths:
                command.extend(("--changed-path", path))
            result = subprocess.run(command, cwd=repo, text=True, capture_output=True)
            if result.returncode:
                raise ValueError("immutable emitting planner failed: " + result.stderr.strip())
            emitting = dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)
            if emitting.get("scope") != "full":
                command.extend(("--run-mode", "full_escalation"))
                result = subprocess.run(command, cwd=repo, text=True, capture_output=True)
                if result.returncode:
                    raise ValueError("immutable emitting full planner failed: " + result.stderr.strip())
                emitting = dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)
    identity_fields = {"planner_config_sha256", "execution_contract"}
    fields = {key for key in oracle if key.startswith(("run_", "needs_"))}
    fields.update({"scope", "selected_capabilities", "required_test_units"})
    if set(key for key in emitting if key.startswith(("run_", "needs_"))) != set(key for key in oracle if key.startswith(("run_", "needs_"))):
        raise ValueError("trusted full-plan selector inventory mismatch")
    for key in fields | identity_fields:
        if plan.get(key) != emitting.get(key):
            raise ValueError("trusted full-plan emitting metadata mismatch: " + key)
    for key in fields:
        if plan.get(key) != oracle[key]:
            raise ValueError("trusted full-plan coverage mismatch: " + key)
        if key.startswith(("run_", "needs_")):
            environment_key = ("OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS"
                               if key == "run_oasis7_workspace_support_crate_tests"
                               else "OASIS7_CI_" + key.upper())
            if (os.environ if execution_environment is None else execution_environment).get(environment_key) != oracle[key]:
                raise ValueError("actual runner full-plan selector mismatch: " + key)
    if (os.environ if execution_environment is None else execution_environment).get("OASIS7_CI_EXECUTION_CONTRACT") != plan.get("execution_contract"):
        raise ValueError("actual runner full-plan execution contract mismatch")
    return natural_full


def run_scope(repo: Path, base: str, head: str, primary: str = "auto", *, json_output: bool = False, trusted_full_plan: str | None = None, execution_environment: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    repo = repo.resolve()
    def git(*args: str) -> str:
        return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()
    target = git("rev-parse", f"{base}^{{commit}}")
    candidate = git("rev-parse", f"{head}^{{commit}}")
    ancestors = git("merge-base", "--all", target, candidate).splitlines()
    if len(ancestors) != 1:
        raise ValueError("Cargo scope requires one unique merge base")
    source = ancestors[0]
    verified_full = bool(trusted_full_plan and verify_full_plan(repo, target, candidate, source, trusted_full_plan, execution_environment))
    paths = changed_paths(repo, source, candidate)
    renames = git("diff", "--name-status", "--find-renames", source, candidate).splitlines()
    maintenance = (verified_full and bool(paths) and set(paths).issubset(POLICY_MAINTENANCE_PATHS)
                   and ".pm/cargo-package-auxiliary-files.json" in paths
                   and not any(line.startswith(("R", "C")) for line in renames))
    if maintenance:
        execution_paths = json.loads(Path(trusted_full_plan).read_text())["changed_paths"].split(";")
        maintenance = set(execution_paths).issubset(POLICY_MAINTENANCE_PATHS)
    # Materialize all trusted scripts rather than guessing the transitive import
    # graph. Missing classifier/corpus imports in S fail inside isolated Python.
    with tempfile.TemporaryDirectory(prefix="trusted-cargo-scope-") as directory:
        authority = Path(directory)
        extract_scripts(repo, candidate if maintenance else source, authority)
        checker = authority / "scripts/pm/check-cargo-package-scope"
        if not checker.is_file():
            raise ValueError("trusted source Cargo checker is unavailable")
        command = [sys.executable, "-I", str(checker), "--repo-root", str(repo),
                   "--base", source, "--head", candidate, "--primary-package", primary,
                   "--policy", str(repo / ".pm/cargo-package-scope-policy.json")]
        if json_output:
            command.append("--json")
        authenticated_cli = 'parser.add_argument("--trusted-full-plan")' in checker.read_text()
        if verified_full and authenticated_cli:
            command.extend(("--trusted-full-plan", str(Path(trusted_full_plan).resolve()), "--trusted-planner-base", target))
        environment = dict(os.environ)
        if execution_environment is not None:
            environment.update(execution_environment)
        environment.pop("OASIS7_CARGO_SCOPE_TRUSTED_FULL_PLAN", None)
        if verified_full and not authenticated_cli:
            # Historical checkers consume this marker. Only B-derived verified
            # coverage and actual runner selectors may produce it here.
            environment["OASIS7_CARGO_SCOPE_TRUSTED_FULL_PLAN"] = "true"
        return subprocess.run(command, cwd=repo, env=environment, text=True, capture_output=True, check=False)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True, type=Path)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--primary-package", default="auto")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--trusted-full-plan")
    args = parser.parse_args()
    try:
        result = run_scope(args.repo_root, args.base, args.head, args.primary_package, json_output=args.json, trusted_full_plan=args.trusted_full_plan)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"trusted Cargo scope: {exc}", file=sys.stderr)
        return 2
    print(result.stdout, end="")
    print(result.stderr, end="", file=sys.stderr)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
