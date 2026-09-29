#!/usr/bin/env python3
"""Reject concrete GitHub delivery identities from repository workflow surfaces."""
from __future__ import annotations

import argparse
import ast
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys
from typing import Iterable


SURFACE_DIRS = (
    "doc/engineering/workflow",
    "doc/engineering/doc-governance",
    "doc/engineering/governance",
    ".agents",
    ".codex",
    ".github",
    "scripts/pm",
    ".pm/templates",
)
SURFACE_FILES = (
    ".pm/.gitignore",
    ".pm/README.md",
    ".pm/cargo-package-scope-policy.json",
    ".pm/registry/roles.yaml",
    "scripts/full-escalation-receipt.py",
    "scripts/ci-required-capability-test-inventory.tsv",
    "scripts/ci-required-scope.v2.json",
    "scripts/ci-tests.sh",
    "scripts/ci-tests-argument-contract.test.sh",
    "scripts/ci-tests-full-superset-contract.test.sh",
    "scripts/doc-governance-check.sh",
    "scripts/doc-governance-check.test.sh",
    "scripts/lint-skills.sh",
    "scripts/new-task-worktree.sh",
    "scripts/plan-rust-required-scope.py",
    "scripts/plan-rust-required-scope.test.sh",
    "scripts/pre-commit.sh",
    "scripts/pre-commit.test.sh",
    "scripts/prepare-task-pr.sh",
    "scripts/prepare-task-pr.test.sh",
    "scripts/rust-required-gate-apt-contract.test.sh",
    "scripts/rust-required-gate-compile-command-contract.test.sh",
    "scripts/rust-full-tier-trunk-prerequisite-contract.test.sh",
    "scripts/worktree-harness-lib.sh",
    "scripts/workflow-process-identity-check.py",
    "scripts/workflow-process-identity-check.test.py",
)
SURFACE_ROOT_FILES = ("AGENTS.md", "README.md", "CONTRIBUTING.md")
TEXT_SUFFIXES = frozenset({
    ".cfg", ".ini", ".js", ".jsx", ".json", ".jsonl", ".md", ".py", ".sh",
    ".toml", ".ts", ".tsx", ".tsv", ".txt", ".yml", ".yaml",
})
SKIP_DIRS = frozenset({".git", "node_modules", "target", "build", "dist", ".venv"})
_PM_PROCESS_STATE_FILES = frozenset({
    ".pm/github-project-sync/signal-archive.jsonl",
    ".pm/github-project-sync/task-archive.jsonl",
    ".pm/github-project-sync/task-retirement-summary.json",
    ".pm/registry/codex-sessions.yaml",
    ".pm/stage/current.yaml",
    ".pm/stage/gate.yaml",
})
_PM_PROCESS_STATE_PREFIXES = (
    ".pm/scratch/", ".pm/working_memory/", ".pm/cache/", ".pm/runtime/",
)
_PM_ROLE_BACKLOG_RE = re.compile(r"\.pm/roles/[^/]+/backlog/.+")
_PM_ROLE_MEMORY_RE = re.compile(
    r"\.pm/(?:roles/[^/]+/memory|shared/memory)/[^/]+\.(?:yaml|yml|md)\Z",
)

_TASK_UID_RE = re.compile(r"(?<![A-Za-z0-9_])task_[0-9a-f]{32}(?![A-Za-z0-9_])")
_SHA_RE = re.compile(r"(?<![0-9a-fA-F])[0-9a-fA-F]{40}(?![0-9a-fA-F])")
_GITHUB_PROCESS_URL_RE = re.compile(
    r"https?://(?:www\.)?(?:github\.com|api\.github\.com)/"
    r"(?:repos/)?(?P<owner>[A-Za-z0-9_.-]+)/(?P<repo>[A-Za-z0-9_.-]+)/"
    r"(?:issues/(?:comments/)?[1-9][0-9]*|pulls?/(?:comments/|reviews/)?[1-9][0-9]*|"
    r"actions/runs/[1-9][0-9]*|check-runs/[1-9][0-9]*|commits/[0-9a-fA-F]{7,40})"
    r"(?:#[A-Za-z0-9_.-]+)?",
    re.IGNORECASE,
)
_GITHUB_COMMIT_BLOB_URL_RE = re.compile(
    r"https?://(?:www\.)?github\.com/"
    r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/"
    r"(?:commit|commits|blob)/(?P<oid>[0-9a-fA-F]{7,40})(?:/|$)",
    re.IGNORECASE,
)
_PROCESS_FIELD_RE = re.compile(
    r"(?i)\b(?:task_issue_number|issue_number|pr_number|pull_request_number|"
    r"issue_comment_id|comment_id|workflow_run_id|check_run_id|run_id|check_id|review_id)"
    r"\s*[\"']?\s*[:=]\s*[\"']?\s*#?([1-9][0-9]*)\b"
)
_PROCESS_LABEL_RE = re.compile(
    r"(?i)\b(?:issue|pr|pull request|comment|workflow run|check run)"
    r"(?:\s+(?:number|id))?\s*(?:#|:|=)\s*#?([1-9][0-9]*)(?![-A-Za-z0-9_])"
)
_BARE_HASH_ID_RE = re.compile(r"(?<![A-Za-z0-9_#])#([1-9][0-9]{1,})(?![-A-Za-z0-9_])")
_SHA_AUTHORITY_CONTEXT_RE = re.compile(
    r"(?i)\b(?:source|execution|head|base|merge|integration|tested|workflow|run|check|"
    r"evidence|task|review|scope)(?:[ _-]+[a-z]+){0,3}[ _-]+(?:commit|oid|sha|hash)\b"
    r"|\bfixed[ _-]+source[ _-]+baseline\b"
    r"|\b(?:commit|head|base|merge|run|check|evidence)[ _-]+(?:oid|sha|hash)\b"
)
_TEST_FILE_RE = re.compile(
    r"(?:^|/)(?:tests?/|fixtures?/|[^/]+\.test\.[^.]+$|test_[^/]+\.[^.]+$|"
    r"[^/]+_test(?:_support)?\.[^.]+$|[^/]+[-_]smoke\.[^.]+$|[^/]+\.smoke\.[^.]+$)",
    re.IGNORECASE,
)
_TEST_ONLY_IMPORT_COMPONENT_RE = re.compile(
    r"(?i)^(?:tests?|fixtures?|test_[a-z0-9_]*|[a-z0-9_]*_test(?:_support)?)$"
)


@dataclass(frozen=True, order=True)
class Finding:
    path: str
    line: int
    kind: str
    value: str


def _is_test_surface(relative: str) -> bool:
    path = Path(relative)
    return bool(_TEST_FILE_RE.search(relative)) or any(
        part.lower() in {"test", "tests", "fixture", "fixtures", "smoke"}
        for part in path.parts[:-1]
    )


def _test_only_import_findings(root: Path, path: Path, relative: str, source: str) -> list[Finding]:
    """Production PM modules must not import data/code from isolated test surfaces."""
    if not relative.startswith("scripts/pm/") or _is_test_surface(relative):
        return []
    try:
        tree = ast.parse(source, filename=relative)
    except SyntaxError:
        return []  # Syntax errors are reported by the language checkers, not this guard.
    findings: list[Finding] = []
    for node in ast.walk(tree):
        module_names: list[str] = []
        if isinstance(node, ast.Import):
            module_names.extend(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            module_names.append(node.module)
        for module_name in module_names:
            components = module_name.split(".")
            if any(_TEST_ONLY_IMPORT_COMPONENT_RE.fullmatch(part) for part in components):
                findings.append(Finding(relative, node.lineno, "test_only_import", module_name))
    return findings


def _tracked_pm_paths(root: Path) -> tuple[set[str] | None, str | None]:
    """Read the index only; ignored local PM cache files stay outside this guard."""
    if not (root / ".git").exists():
        return None, None
    try:
        raw = subprocess.check_output(
            ["git", "-C", str(root), "ls-files", "-z", "--", ".pm"],
            stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError):
        return set(), "git ls-files could not enumerate tracked .pm paths"
    try:
        paths = {item.decode("utf-8", "strict") for item in raw.split(b"\0") if item}
    except UnicodeDecodeError:
        return set(), "tracked .pm path is not valid UTF-8"
    return paths, None


def _is_static_pm_surface(relative: str) -> bool:
    return (relative in {
        ".pm/.gitignore", ".pm/README.md", ".pm/cargo-package-scope-policy.json",
        ".pm/registry/roles.yaml",
    } or relative.startswith(".pm/templates/") or bool(_PM_ROLE_MEMORY_RE.fullmatch(relative)))


def _surface_files(root: Path, tracked_pm_paths: set[str] | None) -> Iterable[Path]:
    seen: set[Path] = set()
    for relative in (*SURFACE_ROOT_FILES, *SURFACE_FILES):
        path = root / relative
        if relative.startswith(".pm/") and (
                tracked_pm_paths is None or relative not in tracked_pm_paths):
            continue
        if path.is_file() and not path.is_symlink() and path not in seen:
            seen.add(path)
            yield path
    for relative in SURFACE_DIRS:
        directory = root / relative
        if not directory.is_dir():
            continue
        for path in directory.rglob("*"):
            if not path.is_file() or path.is_symlink() or path.suffix.lower() not in TEXT_SUFFIXES:
                continue
            if any(part in SKIP_DIRS for part in path.relative_to(root).parts):
                continue
            relative = path.relative_to(root).as_posix()
            if relative.startswith(".pm/") and (
                    tracked_pm_paths is None or relative not in tracked_pm_paths):
                continue
            if path not in seen:
                seen.add(path)
                yield path
    if tracked_pm_paths is not None:
        for relative in sorted(tracked_pm_paths):
            if not _is_static_pm_surface(relative):
                continue
            path = root / relative
            if path.is_file() and not path.is_symlink() and path.suffix.lower() in TEXT_SUFFIXES:
                if path not in seen:
                    seen.add(path)
                    yield path


def _tracked_pm_state_findings(
    tracked_pm_paths: set[str] | None, inventory_error: str | None,
) -> list[Finding]:
    if inventory_error:
        return [Finding(".pm", 1, "tracked_pm_inventory_unavailable", inventory_error)]
    if tracked_pm_paths is None:
        return []
    findings = []
    for relative in sorted(tracked_pm_paths):
        basename = relative.rsplit("/", 1)[-1]
        process_state = (
            relative in _PM_PROCESS_STATE_FILES
            or (relative.startswith(_PM_PROCESS_STATE_PREFIXES) and basename != ".gitkeep")
            or (bool(_PM_ROLE_BACKLOG_RE.fullmatch(relative)) and basename != ".gitkeep")
        )
        if process_state:
            findings.append(Finding(
                relative, 1, "tracked_pm_process_state",
                "runtime Task, signal, archive, or backlog data must remain out of Git",
            ))
    return findings


def scan_repository(repo_root: str | Path) -> list[Finding]:
    """Scan documented active workflow surfaces and return stable, line-level findings."""
    root = Path(repo_root).resolve()
    findings: list[Finding] = []
    tracked_pm_paths, inventory_error = _tracked_pm_paths(root)
    findings.extend(_tracked_pm_state_findings(tracked_pm_paths, inventory_error))
    for path in _surface_files(root, tracked_pm_paths):
        relative = path.relative_to(root).as_posix()
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            findings.append(Finding(relative, 1, "unreadable_surface", str(exc)))
            continue
        lines = text.splitlines()
        test_surface = _is_test_surface(relative)
        findings.extend(_test_only_import_findings(root, path, relative, text))
        for line_number, line in enumerate(lines, start=1):
            # JSON/YAML fixtures often escape slashes; normalize only that URL spelling
            # before matching, while retaining the original line and match evidence.
            normalized_line = line.replace(r"\/", "/")
            if not test_surface:
                for match in _GITHUB_PROCESS_URL_RE.finditer(normalized_line):
                    findings.append(Finding(relative, line_number, "github_process_url", match.group(0)))
                for match in _GITHUB_COMMIT_BLOB_URL_RE.finditer(normalized_line):
                    findings.append(Finding(relative, line_number, "github_commit_or_blob_ref", match.group(0)))

            for match in _TASK_UID_RE.finditer(line):
                value = match.group(0)
                if not test_surface:
                    findings.append(Finding(relative, line_number, "task_uid", value))

            for pattern, kind in (
                (_PROCESS_FIELD_RE, "process_id_field"),
                (_PROCESS_LABEL_RE, "process_id_reference"),
                (_BARE_HASH_ID_RE, "numbered_process_reference"),
            ):
                for match in pattern.finditer(line):
                    if test_surface:
                        continue
                    findings.append(Finding(relative, line_number, kind, match.group(0)))

            if not test_surface:
                context = "\n".join(lines[max(0, line_number - 1): min(len(lines), line_number + 1)])
                if _SHA_AUTHORITY_CONTEXT_RE.search(context):
                    for match in _SHA_RE.finditer(line):
                        findings.append(Finding(relative, line_number, "execution_commit_hash", match.group(0)))

    return sorted(set(findings))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root", type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="repository root to scan (defaults to this script's repository)",
    )
    args = parser.parse_args(argv)
    findings = scan_repository(args.repo_root)
    if findings:
        for finding in findings:
            print(
                f"workflow-process-identity-check: {finding.path}:{finding.line}: "
                f"{finding.kind}: {finding.value}",
                file=sys.stderr,
            )
        print(
            f"workflow-process-identity-check: FAIL ({len(findings)} process identity finding(s))",
            file=sys.stderr,
        )
        return 1
    print("workflow-process-identity-check: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
