#!/usr/bin/env python3
"""Gate newly changed professional system-design traceability.

This is intentionally a structural admission check.  It verifies that a
changed design hands an exact upstream requirement to a local design fragment
and gives that fragment an exact validation source (or a complete, explicit
N/A disposition).  Semantic design quality remains the responsibility of the
professional review roles.
"""

from __future__ import annotations

import argparse
from collections.abc import Callable
from dataclasses import dataclass
import html
import posixpath
from pathlib import Path
import re
import subprocess
import sys
from typing import Optional
from urllib.parse import unquote

try:
    from product_doc_markdown import parse_markdown_blocks, parse_markdown_html, parse_markdown_links
except RuntimeError as exc:
    print(f"system-design-traceability: error: {exc}", file=sys.stderr)
    raise SystemExit(2) from exc


DESIGN_SUFFIX = ".design.md"
AUTHORITY_SUFFIXES = (".prd.md", DESIGN_SUFFIX)
PRODUCT_ROOT = Path("doc/product")
DESIGN_STANDARD_EXCLUSIONS = frozenset(
    {
        "doc/engineering/doc-governance/doc-structure-standard.design.md",
        "doc/engineering/doc-governance/product-documentation-full-corpus-governance.design.md",
        "doc/engineering/doc-governance/product-documentation-standard.design.md",
        "doc/engineering/doc-governance/project-management-record-standard.design.md",
        "doc/engineering/doc-governance/system-design-writing-standard.design.md",
    }
)
DEMAND_HEADING = re.compile(r"^\s*###\s+2\.1\s+需求承接与分配表\s*$", re.IGNORECASE)
VALIDATION_HEADING = re.compile(r"^\s*###\s+11\.1\s+验证映射表\s*$", re.IGNORECASE)
HEADING_RE = re.compile(r"^ {0,3}#{1,6}\s+")
HTML_COMMENT_RE = re.compile(r"<!--[\s\S]*?-->")
REQUIRED_NA_FIELDS = ("reason", "scope", "owner_role", "evidence_ref", "re-evaluate")
DEMAND_ROW_CELLS = 5
VALIDATION_ROW_CELLS = 6
CANONICAL_GITHUB_EVIDENCE_RE = re.compile(
    r"^https://github\.com/eng-cc/oasis7/issues/(?P<issue>[1-9][0-9]*)#issuecomment-(?P<comment>[1-9][0-9]*)$"
)


@dataclass(frozen=True)
class Table:
    heading_line: int
    header_line: int
    headers: tuple[str, ...]
    rows: tuple[tuple[int, tuple[str, ...]], ...]


@dataclass(frozen=True)
class Relation:
    upstream_path: str
    upstream_fragment: str
    local_fragment: str
    line: int


TargetTextReader = Callable[[Path], Optional[str]]


def fail(errors: list[str], code: str, path: str, detail: str) -> None:
    errors.append(f"system-design-traceability: {code}: {path}: {detail}")


def run_git(root: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise ValueError((result.stderr or result.stdout).strip() or "git command failed")
    return result.stdout


def resolve_commit(root: Path, value: str | None, label: str) -> str:
    if not value:
        raise ValueError(f"missing {label} commit OID")
    if not re.fullmatch(r"[0-9a-fA-F]{40}", value):
        raise ValueError(f"{label} must be a full 40-character commit OID")
    try:
        resolved = run_git(root, "rev-parse", "--verify", f"{value}^{{commit}}").strip()
    except ValueError as exc:
        raise ValueError(f"invalid {label} commit OID {value!r}: {exc}") from exc
    if not re.fullmatch(r"[0-9a-f]{40}", resolved):
        raise ValueError(f"{label} did not resolve to a full commit OID")
    return resolved


def is_system_design_path(path: str) -> bool:
    """Return whether a path is a governed professional/system design.

    Product-layer designs have their own content gate.  The five governance
    standards listed above are normative overlays, not system designs; the
    cross-layer contract and system-design standard remain governed here.
    """
    normalized = Path(path).as_posix()
    return (
        normalized.endswith(DESIGN_SUFFIX)
        and not normalized.startswith(f"{PRODUCT_ROOT.as_posix()}/")
        and normalized not in DESIGN_STANDARD_EXCLUSIONS
    )


def is_traceability_authority_path(path: str) -> bool:
    """Return whether a changed document can be consumed as an authority.

    This is deliberately a path-level classification, not a caller-provided
    review class.  Product PRDs and professional design/PRD documents are
    the only bounded authority endpoints considered by reverse selection.
    """
    normalized = Path(path).as_posix()
    return (
        normalized.startswith("doc/")
        and normalized.endswith(AUTHORITY_SUFFIXES)
        and normalized not in DESIGN_STANDARD_EXCLUSIONS
    )


def parse_name_status(output: str) -> dict[str, str]:
    changed: dict[str, str] = {}
    for line in output.splitlines():
        if not line.strip():
            continue
        fields = line.split("\t")
        status = fields[0]
        if status.startswith(("R", "C")):
            if len(fields) >= 3:
                changed[fields[2]] = status
            continue
        if len(fields) >= 2:
            changed[fields[1]] = status
    return changed


def without_html_comments(text: str) -> str:
    def preserve_newlines(match: re.Match[str]) -> str:
        return "".join(character for character in match.group(0) if character in "\r\n")

    return HTML_COMMENT_RE.sub(preserve_newlines, text)


def normalized_for_change(text: str | None) -> str:
    if text is None:
        return ""
    text = without_html_comments(text).replace("\r\n", "\n").replace("\r", "\n")
    return "\n".join(line.rstrip() for line in text.splitlines() if line.strip())


def git_text(root: Path, commit: str, path: str) -> str | None:
    result = subprocess.run(
        ["git", "-C", str(root), "show", f"{commit}:{path}"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def git_index_text(root: Path, path: str) -> str | None:
    result = subprocess.run(
        ["git", "-C", str(root), "show", f":{path}"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def git_tree_entry(root: Path, commit: str, path: str) -> tuple[str, str] | None:
    result = subprocess.run(
        ["git", "-C", str(root), "ls-tree", "-z", commit, "--", path],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not result.stdout:
        return None
    record = result.stdout.split("\0", 1)[0]
    metadata, separator, tree_path = record.partition("\t")
    fields = metadata.split()
    if not separator or len(fields) < 1:
        return None
    return fields[0], tree_path


def committed_target_text(root: Path, commit: str, target: Path) -> str | None:
    """Read a trusted-head target while fail-closing unsafe symlinks.

    ``git show COMMIT:path`` returns the link payload for a symlink rather than
    the target file.  Resolve committed symlinks from the Git tree instead so
    dangling, escaping, and cyclic links cannot masquerade as validation
    sources.  In-repository symlinks remain usable when their full target
    chain resolves to a committed blob.
    """
    try:
        current = target.relative_to(root).as_posix()
    except ValueError:
        return None
    visited: set[str] = set()
    while current not in visited:
        visited.add(current)
        entry = git_tree_entry(root, commit, current)
        if entry is None:
            return None
        mode, _tree_path = entry
        if mode != "120000":
            return git_text(root, commit, current)
        link_target = git_text(root, commit, current)
        if link_target is None:
            return None
        link_target = link_target.rstrip("\r\n")
        candidate = posixpath.normpath(posixpath.join(posixpath.dirname(current), link_target))
        if (
            posixpath.isabs(candidate)
            or candidate == ".."
            or candidate.startswith("../")
        ):
            return None
        current = candidate
    return None


def worktree_file_text(root: Path, checkout_head: str, path: str) -> str | None:
    """Read the effective worktree content, preserving staged-only changes.

    ``git diff HEAD`` reports the union of index and worktree changes, but a
    staged tracked file can have its worktree restored to HEAD.  In that case
    the filesystem is intentionally stale relative to the staged candidate,
    so the index blob is the content that the worktree gate must inspect.
    An unstaged change remains authoritative when both index and worktree
    contain edits because it is the content the local gate will observe.
    """
    target = root / path
    unstaged = parse_name_status(run_git(root, "diff", "--name-status", "-M", "--", "."))
    staged = parse_name_status(
        run_git(root, "diff", "--cached", "--name-status", "-M", checkout_head, "--", ".")
    )
    if path in unstaged:
        status = unstaged[path]
        current = target.read_text(encoding="utf-8") if not status.startswith("D") and target.is_file() else None
        staged_status = staged.get(path)
        if staged_status is not None and not staged_status.startswith("D"):
            head_text = git_text(root, checkout_head, path)
            if current == head_text:
                return git_index_text(root, path)
        return current
    if path in staged:
        status = staged[path]
        return git_index_text(root, path) if not status.startswith("D") else None

    untracked = {
        candidate.strip()
        for candidate in run_git(root, "ls-files", "--others", "--exclude-standard", "--", ".").splitlines()
        if candidate.strip()
    }
    if path in untracked:
        return target.read_text(encoding="utf-8") if target.is_file() else None
    return target.read_text(encoding="utf-8") if target.is_file() else None


def worktree_overlay(root: Path, checkout_head: str) -> dict[str, str]:
    overlay = parse_name_status(
        run_git(root, "diff", "--name-status", "-M", checkout_head, "--", ".")
    )
    overlay.update(
        parse_name_status(
            run_git(root, "diff", "--cached", "--name-status", "-M", checkout_head, "--", ".")
        )
    )
    for path in run_git(root, "ls-files", "--others", "--exclude-standard", "--", ".").splitlines():
        if path.strip():
            overlay[path.strip()] = "??"
    return overlay


def worktree_authority_overlay(root: Path, checkout_head: str) -> dict[str, str]:
    """Return worktree changes without rename coalescing.

    A rename is intentionally represented as a deleted old endpoint and an
    added new endpoint so consumers of either frozen path are re-evaluated.
    """
    overlay = parse_name_status(
        run_git(root, "diff", "--name-status", "--no-renames", checkout_head, "--", ".")
    )
    overlay.update(
        parse_name_status(
            run_git(root, "diff", "--cached", "--name-status", "--no-renames", checkout_head, "--", ".")
        )
    )
    for path in run_git(root, "ls-files", "--others", "--exclude-standard", "--", ".").splitlines():
        if path.strip():
            overlay[path.strip()] = "??"
    return overlay


def changed_system_design_paths(
    root: Path,
    base: str,
    head: str,
    include_worktree: bool,
) -> list[Path]:
    """Select changed system designs from an explicit trusted range.

    ``merge-base(base, head)`` is the source baseline.  Consequently content
    that exists only on the target/base side is not attributed to the source.
    Worktree overlay is additive and includes staged, unstaged, and untracked
    files relative to the checked-out HEAD.
    """
    base_oid = resolve_commit(root, base, "base")
    head_oid = resolve_commit(root, head, "head")
    source_base = run_git(root, "merge-base", base_oid, head_oid).strip()
    if not source_base:
        raise ValueError(f"base/head have no merge-base: {base_oid} {head_oid}")
    changed = parse_name_status(
        run_git(root, "diff", "--name-status", "-M", source_base, head_oid, "--", ".")
    )
    if include_worktree:
        checkout_head = run_git(root, "rev-parse", "--verify", "HEAD^{commit}").strip()
        changed.update(worktree_overlay(root, checkout_head))
    else:
        checkout_head = ""

    selected: list[Path] = []
    for path, status in sorted(changed.items()):
        if not is_system_design_path(path) or status.startswith("D"):
            continue
        old_text = git_text(root, source_base, path)
        target = root / path
        new_text = (
            worktree_file_text(root, checkout_head, path)
            if include_worktree
            else git_text(root, head_oid, path)
        )
        if new_text is None:
            continue
        if status.startswith(("A", "R", "C", "??")) or normalized_for_change(old_text) != normalized_for_change(new_text):
            selected.append(target)
    return selected


def changed_authority_paths(
    root: Path,
    base: str,
    head: str,
    include_worktree: bool,
) -> list[str]:
    """Select changed authority endpoints, preserving both rename endpoints."""
    base_oid = resolve_commit(root, base, "base")
    head_oid = resolve_commit(root, head, "head")
    source_base = run_git(root, "merge-base", base_oid, head_oid).strip()
    if not source_base:
        raise ValueError(f"base/head have no merge-base: {base_oid} {head_oid}")
    changed = parse_name_status(
        run_git(root, "diff", "--name-status", "--no-renames", source_base, head_oid, "--", ".")
    )
    checkout_head = ""
    if include_worktree:
        checkout_head = run_git(root, "rev-parse", "--verify", "HEAD^{commit}").strip()
        changed.update(worktree_authority_overlay(root, checkout_head))

    selected: list[str] = []
    for path, status in sorted(changed.items()):
        if not is_traceability_authority_path(path):
            continue
        old_text = git_text(root, source_base, path)
        if status.startswith("D"):
            selected.append(path)
            continue
        new_text = worktree_file_text(root, checkout_head, path) if include_worktree else git_text(root, head_oid, path)
        if new_text is None:
            selected.append(path)
        elif status.startswith(("A", "R", "C", "??")) or normalized_for_change(old_text) != normalized_for_change(new_text):
            selected.append(path)
    return selected


def current_design_paths(root: Path, head: str, include_worktree: bool, checkout_head: str) -> list[Path]:
    """Enumerate current design consumers without broadening to arbitrary docs."""
    paths = {
        line.strip()
        for line in run_git(root, "ls-tree", "-r", "--name-only", head).splitlines()
        if line.strip() and line.strip().endswith(DESIGN_SUFFIX)
    }
    if include_worktree:
        paths.update(
            path
            for path, status in worktree_authority_overlay(root, checkout_head).items()
            if not status.startswith("D") and path.endswith(DESIGN_SUFFIX)
        )
    return [root / path for path in sorted(paths) if is_system_design_path(path)]


def is_traceability_consumer(text: str) -> bool:
    """Recognize a current structured consumer while excluding legacy prose."""
    demand = table_after_heading(text, DEMAND_HEADING)
    validation = table_after_heading(text, VALIDATION_HEADING)
    return bool(
        (demand is not None and demand.rows)
        or (validation is not None and validation.rows)
    )


def references_authority(
    root: Path,
    design: Path,
    text: str,
    authority_paths: set[str],
    *,
    follow_symlinks: bool,
) -> bool:
    """Match exact repository-relative link endpoints, including deleted ones."""
    for node in parse_markdown_links(without_html_comments(text)):
        target, _fragment = split_link_target(node.target)
        if not target or external_target(target):
            continue
        resolved = resolve_target(root, design, target, follow_symlinks=follow_symlinks)
        if resolved is None:
            continue
        try:
            relative = resolved.relative_to(root).as_posix()
        except ValueError:
            continue
        if relative in authority_paths:
            return True
    return False


def authority_impacted_design_paths(
    root: Path,
    head: str,
    authority_paths: list[str],
    *,
    include_worktree: bool,
    checkout_head: str,
    read_target: TargetTextReader,
) -> list[Path]:
    """Reverse-select bounded, structured consumers of changed authorities."""
    impacted: list[Path] = []
    changed = set(authority_paths)
    if not changed:
        return impacted
    for design in current_design_paths(root, head, include_worktree, checkout_head):
        text = read_target(design)
        if text is None or not is_traceability_consumer(text):
            continue
        if references_authority(
            root,
            design,
            text,
            changed,
            follow_symlinks=include_worktree,
        ):
            impacted.append(design)
    return impacted


def visible_lines(text: str) -> list[tuple[int, str]]:
    text = without_html_comments(text)
    excluded_lines = {
        number
        for block in parse_markdown_blocks(text)
        for number in range(block.start_line, block.end_line + 1)
    }
    return [
        (number, line)
        for number, line in enumerate(text.splitlines(), start=1)
        if number not in excluded_lines
    ]


def table_cells(line: str) -> tuple[str, ...]:
    stripped = line.strip()
    if not stripped.startswith("|"):
        return ()
    if stripped.endswith("|"):
        stripped = stripped[:-1]
    cells = stripped[1:].split("|")
    return tuple(cell.strip() for cell in cells)


def is_separator(line: str) -> bool:
    cells = table_cells(line)
    return bool(cells) and all(bool(re.fullmatch(r":?\s*-{3,}\s*:?", cell)) for cell in cells)


def normalized_header(value: str) -> str:
    value = re.sub(r"`([^`]*)`", r"\1", html.unescape(value))
    return re.sub(r"\s+", " ", value).strip().casefold()


def has_header(table: Table, index: int, *markers: str) -> bool:
    if index >= len(table.headers):
        return False
    header = normalized_header(table.headers[index])
    return any(marker.casefold() in header for marker in markers)


def is_demand_table(table: Table) -> bool:
    return (
        len(table.headers) >= 5
        and has_header(table, 0, "上游", "upstream", "requirement")
        and has_header(table, 2, "本设计", "local design", "design")
    )


def is_validation_table(table: Table) -> bool:
    return (
        len(table.headers) >= 6
        and has_header(table, 0, "上游", "upstream", "requirement")
        and has_header(table, 1, "本设计", "local design", "design")
        and has_header(table, 3, "验证", "validation", "test", "manual")
    )


def table_after_heading(text: str, matcher: re.Pattern[str]) -> Table | None:
    lines = visible_lines(text)
    heading_line = None
    start_index = None
    for index, (number, line) in enumerate(lines):
        if matcher.fullmatch(line):
            heading_line = number
            start_index = index + 1
            break
    if start_index is None or heading_line is None:
        return None
    for index in range(start_index, len(lines) - 1):
        number, line = lines[index]
        if HEADING_RE.match(line):
            break
        headers = table_cells(line)
        if not headers or not is_separator(lines[index + 1][1]):
            continue
        rows: list[tuple[int, tuple[str, ...]]] = []
        row_index = index + 2
        while row_index < len(lines):
            row_number, row_line = lines[row_index]
            cells = table_cells(row_line)
            if not cells:
                break
            rows.append((row_number, cells))
            row_index += 1
        return Table(heading_line, number, headers, tuple(rows))
    return None


def split_link_target(raw: str) -> tuple[str, str | None]:
    target = raw.strip().split(None, 1)[0].strip("<>")
    if "#" not in target:
        return target, None
    path, fragment = target.split("#", 1)
    return path, unquote(fragment)


def external_target(target: str) -> bool:
    return target.startswith(("mailto:", "//")) or "://" in target


def github_heading_slug(value: str) -> str:
    value = re.sub(r"<[^>]+>", "", html.unescape(value)).strip().lower()
    value = re.sub(r"[`*_~]", "", value)
    value = re.sub(r"[^\w\-\u0080-\uffff ]", "", value, flags=re.UNICODE)
    return re.sub(r"\s+", "-", value)


def fragment_occurrences(text: str, fragment: str) -> int:
    wanted = fragment.strip().casefold()
    anchor_occurrences = sum(
        anchor.strip().casefold() == wanted
        for node in parse_markdown_html(without_html_comments(text))
        for anchor in re.findall(r"<a\s+[^>]*\bid\s*=\s*[\"']([^\"']+)[\"'][^>]*>", node.content, re.IGNORECASE)
    )
    heading_occurrences = 0
    for _number, line in visible_lines(text):
        match = re.match(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$", line)
        if match and github_heading_slug(match.group(1)) == wanted:
            heading_occurrences += 1
    return anchor_occurrences + heading_occurrences


def resolve_target(root: Path, source: Path, target: str, *, follow_symlinks: bool) -> Path | None:
    """Resolve a repository-relative link without weakening trusted-head reads."""
    root = root.resolve()
    try:
        source_relative = source.absolute().relative_to(root)
    except ValueError:
        return None
    candidate_relative = posixpath.normpath(
        posixpath.join(source_relative.parent.as_posix(), target)
    )
    if (
        posixpath.isabs(candidate_relative)
        or candidate_relative == ".."
        or candidate_relative.startswith("../")
    ):
        return None
    candidate = root / Path(candidate_relative)
    if not follow_symlinks:
        return candidate
    try:
        resolved = candidate.resolve(strict=False)
        resolved.relative_to(root)
    except (OSError, ValueError):
        return None
    return resolved


def link_nodes(cell: str) -> list[str]:
    return [link.target for link in parse_markdown_links(cell)]


def validate_reference(
    root: Path,
    source: Path,
    raw_target: str,
    *,
    path: str,
    code: str,
    errors: list[str],
    require_fragment: bool,
    markdown_target: bool,
    follow_symlinks: bool,
    read_target: TargetTextReader,
) -> tuple[Path | None, str | None]:
    target, fragment = split_link_target(raw_target)
    if (not target and fragment is None) or external_target(target) or (require_fragment and not fragment):
        fail(errors, code, path, f"reference must be a repository-relative path#fragment link; repair the table row: {raw_target!r}")
        return None, fragment
    target_path = source if not target else resolve_target(
        root,
        source,
        target,
        follow_symlinks=follow_symlinks,
    )
    if target_path is None:
        fail(errors, code, path, f"reference escapes the repository; repair the path: {raw_target!r}")
        return None, fragment
    target_text = read_target(target_path)
    if target_text is None:
        fail(errors, code, path, f"reference target is missing; repair the path: {target_path.relative_to(root).as_posix()}")
        return None, fragment
    if markdown_target and not target_path.name.endswith(".md"):
        fail(errors, code, path, f"upstream reference must target Markdown; repair the path: {raw_target!r}")
        return None, fragment
    if fragment:
        occurrences = fragment_occurrences(target_text, fragment)
        if occurrences == 0:
            fail(errors, "trace-ref-unresolved", path, f"fragment is unresolved; repair the reference: {target_path.relative_to(root).as_posix()}#{fragment}")
            return None, fragment
        if occurrences > 1:
            fail(errors, "trace-ref-ambiguous", path, f"fragment is ambiguous; repair duplicate anchors/headings: {target_path.relative_to(root).as_posix()}#{fragment}")
            return None, fragment
    return target_path, fragment


def is_test_or_manual_source(path: Path, design: Path) -> bool:
    """Accept only repository-owned test or manual source paths."""
    if path.resolve() == design.resolve() or path.name.casefold().endswith(".design.md"):
        return False
    parts = {part.casefold() for part in path.parts[:-1]}
    name = path.name.casefold()
    if parts.intersection({"test", "tests", "manual", "manuals"}):
        return True
    if name in {"testing-manual.md", "manual.md"} or name.endswith(".manual.md"):
        return True
    return bool(re.search(r"(?:^|[._-])(test|spec)(?:[._-]|$)", name))


def first_valid_link(
    root: Path,
    source: Path,
    cell: str,
    *,
    path: str,
    code: str,
    errors: list[str],
    require_fragment: bool,
    markdown_target: bool,
    follow_symlinks: bool,
    read_target: TargetTextReader,
) -> tuple[Path | None, str | None]:
    targets = link_nodes(cell)
    if not targets:
        fail(errors, code, path, "add an exact repository-relative Markdown link with path#fragment")
        return None, None
    valid: list[tuple[Path, str | None]] = []
    for target in targets:
        resolved, fragment = validate_reference(
            root,
            source,
            target,
            path=path,
            code=code,
            errors=errors,
            require_fragment=require_fragment,
            markdown_target=markdown_target,
            follow_symlinks=follow_symlinks,
            read_target=read_target,
        )
        if resolved is not None:
            valid.append((resolved, fragment))
    if len(valid) > 1:
        fail(errors, "trace-ref-ambiguous", path, "row has multiple valid references; repair it to one exact relation")
        return None, None
    return valid[0] if valid else (None, None)


def relation_from_demand_row(
    root: Path,
    source: Path,
    row_number: int,
    cells: tuple[str, ...],
    errors: list[str],
    follow_symlinks: bool,
    read_target: TargetTextReader,
) -> Relation | None:
    row_path = f"{source.relative_to(root).as_posix()}:{row_number}"
    if len(cells) != DEMAND_ROW_CELLS or any(not cell.strip() for cell in cells):
        fail(
            errors,
            "trace-demand-row-invalid",
            row_path,
            "demand-allocation row must contain exactly five non-empty cells; repair the row shape and every required column",
        )
        return None
    upstream_path, upstream_fragment = first_valid_link(
        root,
        source,
        cells[0],
        path=row_path,
        code="trace-upstream-missing",
        errors=errors,
        require_fragment=True,
        markdown_target=True,
        follow_symlinks=follow_symlinks,
        read_target=read_target,
    )
    local_path, local_fragment = first_valid_link(
        root,
        source,
        cells[2],
        path=row_path,
        code="trace-system-design-missing",
        errors=errors,
        require_fragment=True,
        markdown_target=True,
        follow_symlinks=follow_symlinks,
        read_target=read_target,
    )
    if upstream_path is None or upstream_fragment is None or local_path is None or local_fragment is None:
        return None
    if upstream_path.resolve() == source.resolve():
        fail(errors, "trace-upstream-missing", row_path, "upstream relation must name another exact Markdown path#fragment; repair the first cell")
        return None
    if local_path.resolve() != source.resolve():
        fail(errors, "trace-system-design-missing", row_path, "local design relation must point back to this file#fragment; repair the third cell")
        return None
    return Relation(
        upstream_path.relative_to(root).as_posix(),
        upstream_fragment,
        local_fragment,
        row_number,
    )


def na_is_complete(value: str) -> bool:
    lowered = value.casefold()
    if not lowered.startswith("n/a"):
        return False
    for field in REQUIRED_NA_FIELDS:
        match = re.search(
            rf"(?:^|[:：;；,，])\s*{re.escape(field)}\s*=\s*(?P<value>[^;；,，]+)",
            value,
            re.IGNORECASE,
        )
        if not match or not match.group("value").strip():
            return False
    return True


def na_evidence_ref_is_readable(
    root: Path,
    source: Path,
    raw_value: str,
    *,
    follow_symlinks: bool,
    read_target: TargetTextReader,
) -> bool:
    """Accept a readable repository path#fragment or canonical GitHub locator."""
    value = raw_value.strip()
    if CANONICAL_GITHUB_EVIDENCE_RE.fullmatch(value):
        return True
    links = link_nodes(value)
    if len(links) == 1:
        value = links[0].strip()
    target, fragment = split_link_target(value)
    if not target or not fragment or external_target(target):
        return False
    candidates: list[Path] = []
    source_relative = resolve_target(root, source, target, follow_symlinks=follow_symlinks)
    if source_relative is not None:
        candidates.append(source_relative)
    repository_relative = posixpath.normpath(target)
    if (
        repository_relative not in {"..", "."}
        and not posixpath.isabs(repository_relative)
        and not repository_relative.startswith("../")
    ):
        candidate = root / Path(repository_relative)
        if follow_symlinks:
            try:
                candidate = candidate.resolve(strict=False)
                candidate.relative_to(root.resolve())
            except (OSError, ValueError):
                candidate = None
        if candidate is not None:
            candidates.append(candidate)
    for candidate in candidates:
        text = read_target(candidate)
        if text is not None and fragment_occurrences(text, fragment) == 1:
            return True
    return False


def check_design_content(
    root: Path,
    source: Path,
    text: str,
    read_target: TargetTextReader,
    *,
    follow_symlinks: bool,
) -> list[str]:
    errors: list[str] = []
    relative = source.relative_to(root).as_posix()
    demand = table_after_heading(text, DEMAND_HEADING)
    validation = table_after_heading(text, VALIDATION_HEADING)
    if demand is None or not demand.rows or not is_demand_table(demand):
        fail(errors, "trace-upstream-missing", relative, "add a 2.1 demand-allocation table with exact upstream path#fragment and local design path#fragment relations")
    if validation is None or not validation.rows or not is_validation_table(validation):
        fail(errors, "trace-validation-missing", relative, "add an 11.1 validation mapping table with an exact test/manual source or complete N/A disposition")
    if (
        demand is None
        or validation is None
        or not is_demand_table(demand)
        or not is_validation_table(validation)
    ):
        return errors

    relations: list[Relation] = []
    for row_number, cells in demand.rows:
        relation = relation_from_demand_row(
            root,
            source,
            row_number,
            cells,
            errors,
            follow_symlinks,
            read_target,
        )
        if relation is not None:
            occurrences = fragment_occurrences(text, relation.local_fragment)
            if occurrences == 0:
                fail(errors, "trace-ref-unresolved", f"{relative}:{row_number}", f"local design fragment is unresolved; repair this file#{relation.local_fragment}")
            elif occurrences > 1:
                fail(errors, "trace-ref-ambiguous", f"{relative}:{row_number}", f"local design fragment is ambiguous; repair duplicate anchors/headings: {relation.local_fragment}")
            relations.append(relation)
    if len(relations) != len({(item.upstream_path, item.upstream_fragment, item.local_fragment) for item in relations}):
        fail(errors, "trace-slot-cardinality", relative, "each demand-allocation relation must be unique; repair duplicate rows")

    validation_keys: set[tuple[str, str]] = set()
    for row_number, cells in validation.rows:
        row_path = f"{relative}:{row_number}"
        if len(cells) != VALIDATION_ROW_CELLS or any(not cell.strip() for cell in cells):
            fail(
                errors,
                "trace-validation-row-invalid",
                row_path,
                "validation mapping row must contain exactly six non-empty cells; repair the row shape and every required column",
            )
            continue
        upstream_path, upstream_fragment = first_valid_link(
            root,
            source,
            cells[0],
            path=row_path,
            code="trace-validation-missing",
            errors=errors,
            require_fragment=True,
            markdown_target=True,
            follow_symlinks=follow_symlinks,
            read_target=read_target,
        )
        local_path, local_fragment = first_valid_link(
            root,
            source,
            cells[1],
            path=row_path,
            code="trace-validation-missing",
            errors=errors,
            require_fragment=True,
            markdown_target=True,
            follow_symlinks=follow_symlinks,
            read_target=read_target,
        )
        if upstream_path is None or upstream_fragment is None or local_path is None or local_fragment is None:
            continue
        if local_path.resolve() != source.resolve():
            fail(errors, "trace-validation-missing", row_path, "local design validation mapping must point to this file#fragment; repair the second cell")
            continue
        relation_key = (upstream_path.relative_to(root).as_posix() + f"#{upstream_fragment}", local_fragment)
        if relation_key in validation_keys:
            fail(errors, "trace-slot-cardinality", row_path, "each validation relation must appear exactly once; repair duplicate rows")
        if not any(
            item.upstream_path + f"#{item.upstream_fragment}" == relation_key[0] and item.local_fragment == local_fragment
            for item in relations
        ):
            fail(errors, "trace-validation-missing", row_path, "validation row has no matching demand-allocation relation; repair upstream/local fragments")
        validation_keys.add(relation_key)
        method = cells[3]
        if method.strip().casefold().startswith("n/a"):
            if not na_is_complete(method):
                fail(errors, "trace-na-incomplete", row_path, "complete N/A with reason, scope, owner_role, evidence_ref, and re-evaluate fields; repair the fourth cell")
            else:
                evidence_match = re.search(
                    r"(?:^|[:：;；,，])\s*evidence_ref\s*=\s*(?P<value>[^;；,，]+)",
                    method,
                    re.IGNORECASE,
                )
                if evidence_match is None or not na_evidence_ref_is_readable(
                    root,
                    source,
                    evidence_match.group("value"),
                    follow_symlinks=follow_symlinks,
                    read_target=read_target,
                ):
                    fail(errors, "trace-na-incomplete", row_path, "evidence_ref must resolve to one readable repository path#fragment or canonical eng-cc/oasis7 Issue/comment; repair the fourth cell")
            continue
        source_links = link_nodes(method)
        if not source_links:
            fail(errors, "trace-validation-missing", row_path, "link an exact test/manual source or use a complete N/A disposition; repair the fourth cell")
            continue
        valid_sources = []
        for target in source_links:
            resolved, _fragment = validate_reference(
                root,
                source,
                target,
                path=row_path,
                code="trace-validation-missing",
                errors=errors,
                require_fragment=False,
                markdown_target=False,
                follow_symlinks=follow_symlinks,
                read_target=read_target,
            )
            if resolved is not None and is_test_or_manual_source(resolved, source):
                valid_sources.append(resolved)
            elif resolved is not None:
                fail(
                    errors,
                    "trace-validation-source-invalid",
                    row_path,
                    "validation source must be a repository-owned test/manual file, not a design, product, or other document; repair the fourth cell",
                )
        if len(valid_sources) != 1:
            fail(errors, "trace-validation-source-invalid", row_path, "validation source must resolve to exactly one repository-owned test/manual file; repair the fourth cell")

    # Keep the comparison explicit rather than relying on row order.
    expected_pairs = {
        (item.upstream_path + f"#{item.upstream_fragment}", item.local_fragment)
        for item in relations
    }
    missing = expected_pairs - validation_keys
    extra = validation_keys - expected_pairs
    if missing:
        fail(errors, "trace-validation-missing", relative, "add validation rows for every local design relation: " + ", ".join(f"{upstream}->{local}" for upstream, local in sorted(missing)) + "; repair the validation table")
    if extra:
        fail(errors, "trace-slot-cardinality", relative, "remove validation rows without a demand-allocation relation: " + ", ".join(f"{upstream}->{local}" for upstream, local in sorted(extra)) + "; repair the validation table")
    return errors


def repository_root(path: Path) -> Path:
    for candidate in (path.parent, *path.parents):
        if (candidate / ".git").exists():
            return candidate
    return path.parent


def check_system_design(path: Path) -> list[str]:
    """Check a current-worktree design path for the mechanical relation contract."""
    source = path.resolve()
    root = repository_root(source)
    if not source.is_file():
        return [
            f"system-design-traceability: trace-ref-unresolved: {source}: repair the missing design file"
        ]
    return check_design_content(
        root,
        source,
        source.read_text(encoding="utf-8"),
        lambda target: target.read_text(encoding="utf-8") if target.is_file() else None,
        follow_symlinks=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--worktree", action="store_true")
    args = parser.parse_args()
    root = args.repo_root.resolve()
    if args.worktree and not (args.base and args.head):
        print("system-design-traceability: error: --worktree requires explicit --base and --head commit OIDs")
        return 2
    try:
        base = resolve_commit(root, args.base, "base")
        head = resolve_commit(root, args.head, "head")
        selected = changed_system_design_paths(root, base, head, args.worktree)
        checkout_head = run_git(root, "rev-parse", "--verify", "HEAD^{commit}").strip() if args.worktree else ""
    except ValueError as exc:
        print(f"system-design-traceability: error: {exc}")
        return 2

    def read_target(target: Path) -> str | None:
        if args.worktree:
            try:
                relative = target.relative_to(root).as_posix()
            except ValueError:
                return None
            return worktree_file_text(root, checkout_head, relative)
        try:
            target.relative_to(root)
        except ValueError:
            return None
        return committed_target_text(root, head, target)

    try:
        authority_paths = changed_authority_paths(root, base, head, args.worktree)
        selected.extend(
            authority_impacted_design_paths(
                root,
                head,
                authority_paths,
                include_worktree=args.worktree,
                checkout_head=checkout_head,
                read_target=read_target,
            )
        )
    except ValueError as exc:
        print(f"system-design-traceability: error: {exc}")
        return 2
    selected = sorted(set(selected), key=lambda path: path.relative_to(root).as_posix())
    if not selected:
        print("system-design-traceability: checked 0: reason=no new or substantive system-design changes in selected range")
        return 0

    errors: list[str] = []
    for path in selected:
        text = read_target(path)
        if text is None:
            fail(errors, "trace-ref-unresolved", path.relative_to(root).as_posix(), "design content is missing; repair the selected head/worktree file")
            continue
        errors.extend(
            check_design_content(
                root,
                path,
                text,
                read_target,
                follow_symlinks=args.worktree,
            )
        )
    if errors:
        print("\n".join(errors))
        return 1
    print(f"system-design-traceability: OK (checked {len(selected)} changed/new system designs)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
