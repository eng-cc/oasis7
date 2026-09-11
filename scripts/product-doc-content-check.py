#!/usr/bin/env python3
"""Check changed product-document content against the adopted mechanical contract."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import html
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote

try:
    from product_doc_markdown import parse_markdown_blocks, parse_markdown_links
except RuntimeError as exc:
    print(f"product-doc-content: error: {exc}", file=sys.stderr)
    raise SystemExit(2) from exc


PRODUCT_ROOT = Path("doc/product")
PRODUCT_SUFFIXES = (".prd.md", ".design.md")
HTML_COMMENT_RE = re.compile(r"<!--[\s\S]*?-->")
ID_RE = re.compile(r"\b((?:REQ|AC)-[A-Z0-9][A-Z0-9_*-]*)", re.IGNORECASE)
HEADING_PREFIX_RE = re.compile(r"^ {0,3}#{2,6}\s+")
DECLARATION_PREFIX_RE = re.compile(r"^\s*(?:[-+*]\s+|\|\s*)")
TABLE_ID_CELL_RE = re.compile(r"^\s*((?:REQ|AC)-[A-Z0-9][A-Z0-9_*-]*)\s*$", re.IGNORECASE)
ANCHOR_RE = re.compile(r"<a\s+[^>]*\bid\s*=\s*[\"']([^\"']+)[\"'][^>]*>", re.IGNORECASE)
RAW_PATH_RE = re.compile(r"(?:`|\b)(?:doc|\.\.?/)[^`\s,，。；;）)]+(?:\.md)?(?:#[^`\s,，。；;）)]+)?")
HTML_TAG_RE = re.compile(r"<[^>]+>")


@dataclass(frozen=True)
class ChangedDocument:
    path: str
    status: str
    old_text: str | None
    new_text: str


def fail(errors: list[str], code: str, path: str, detail: str) -> None:
    errors.append(f"product-doc-content: {code}: {path}: {detail}")


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


def is_product_doc(path: str) -> bool:
    return path.startswith(f"{PRODUCT_ROOT.as_posix()}/") and path.endswith(PRODUCT_SUFFIXES)


def parse_name_status(output: str) -> dict[str, str]:
    changed: dict[str, str] = {}
    for line in output.splitlines():
        if not line.strip():
            continue
        fields = line.split("\t")
        status = fields[0]
        if status.startswith("R") or status.startswith("C"):
            if len(fields) >= 3:
                changed[fields[2]] = status
            continue
        if len(fields) >= 2:
            changed[fields[1]] = status
    return changed


def diff_paths(root: Path, base: str, head: str, worktree: bool) -> tuple[dict[str, str], set[str]]:
    source_base = run_git(root, "merge-base", base, head).strip()
    if not source_base:
        raise ValueError(f"base/head have no merge-base: {base} {head}")
    changed = parse_name_status(run_git(root, "diff", "--name-status", "-M", source_base, head, "--", "doc/product"))
    if not worktree:
        return changed, set()
    checkout_head = run_git(root, "rev-parse", "--verify", "HEAD^{commit}").strip()
    overlay = parse_name_status(
        run_git(root, "diff", "--name-status", "-M", checkout_head, "--", "doc/product")
    )
    overlay.update(
        parse_name_status(
            run_git(root, "diff", "--cached", "--name-status", "-M", checkout_head, "--", "doc/product")
        )
    )
    for path in run_git(root, "ls-files", "--others", "--exclude-standard", "--", "doc/product").splitlines():
        if path.strip():
            overlay[path.strip()] = "??"
    for path, status in overlay.items():
        changed[path] = status
    return changed, set(overlay)


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


def current_text(root: Path, head: str, path: str, use_worktree_content: bool) -> str | None:
    if use_worktree_content:
        target = root / path
        if not target.is_file():
            return None
        return target.read_text(encoding="utf-8")
    return git_text(root, head, path)


def without_html_comments(text: str) -> str:
    def preserve_newlines(match: re.Match[str]) -> str:
        return "".join(character for character in match.group(0) if character in "\r\n")

    return HTML_COMMENT_RE.sub(preserve_newlines, text)


def normalized_for_change(text: str | None) -> str:
    if text is None:
        return ""
    text = without_html_comments(text).replace("\r\n", "\n").replace("\r", "\n")
    return "\n".join(line.rstrip() for line in text.splitlines() if line.strip())


def visible_lines(text: str) -> list[tuple[int, str]]:
    """Return line-numbered prose, excluding comments and code blocks."""
    text = without_html_comments(text)
    excluded_lines = {
        number
        for block in parse_markdown_blocks(text)
        for number in range(block.start_line, block.end_line + 1)
    }
    visible: list[tuple[int, str]] = []
    for number, line in enumerate(text.splitlines(), start=1):
        if number in excluded_lines:
            continue
        if line.startswith("\t") or line.startswith("    "):
            continue
        visible.append((number, line))
    return visible


def metadata_value(text: str, label: str) -> str | None:
    match = re.search(rf"^- {re.escape(label)}：(.+)$", text, re.MULTILINE)
    return match.group(1).strip() if match else None


def metadata_any(text: str, labels: tuple[str, ...]) -> str | None:
    for label in labels:
        value = metadata_value(text, label)
        if value:
            return value
    return None


def split_link_target(raw: str) -> tuple[str, str | None]:
    target = raw.strip().split(None, 1)[0].strip("<>")
    if "#" not in target:
        return target, None
    path, fragment = target.split("#", 1)
    return path, unquote(fragment)


def is_external_link_target(target: str) -> bool:
    return "://" in target or target.startswith(("mailto:", "//"))


def github_heading_slug(value: str) -> str:
    value = HTML_TAG_RE.sub("", html.unescape(value)).strip().lower()
    value = re.sub(r"[`*_~]", "", value)
    value = re.sub(r"[^\w\-\u0080-\uffff ]", "", value, flags=re.UNICODE)
    return re.sub(r"\s+", "-", value)


def fragment_exists(text: str, fragment: str) -> bool:
    wanted = fragment.strip().lower()
    visible = "\n".join(line for _, line in visible_lines(text))
    for anchor in ANCHOR_RE.findall(visible):
        if anchor.strip().lower() == wanted:
            return True
    for line in visible.splitlines():
        match = re.match(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$", line)
        if match and github_heading_slug(match.group(1)) == wanted:
            return True
    return False


def validate_link(root: Path, source: Path, raw_target: str, source_text: str, errors: list[str], path: str, code: str) -> tuple[Path | None, str | None]:
    target, fragment = split_link_target(raw_target)
    if not target and fragment is not None:
        target_path = source
    elif not target or "://" in target or target.startswith("mailto:"):
        return None, fragment
    else:
        target_path = (source.parent / target).resolve()
    try:
        target_rel = target_path.relative_to(root)
    except ValueError:
        fail(errors, code, path, f"link escapes repository: {raw_target}")
        return None, fragment
    if not target_path.is_file():
        fail(errors, code, path, f"missing target: {target_rel.as_posix()}")
        return None, fragment
    target_text = source_text if target_path == source else target_path.read_text(encoding="utf-8")
    if fragment and not fragment_exists(target_text, fragment):
        fail(errors, "invalid-fragment", path, f"{target_rel.as_posix()}#{fragment}")
    return target_path, fragment


def markdown_links(text: str) -> list[tuple[int, str, str]]:
    return [(link.line, link.target, link.target) for link in parse_markdown_links(text)]


def authority_lines(lines: list[tuple[int, str]]) -> list[tuple[int, str]]:
    return [
        (number, line)
        for number, line in lines
        if re.search(r"专业(?:域)?权威\s*[:：]", line)
    ]


def id_tokens(text: str) -> set[str]:
    tokens: set[str] = set()
    for match in ID_RE.finditer(text):
        token = match.group(1).upper()
        # A wildcard such as REQ-TOPIC-* is prose describing an ID family,
        # not a declaration or a cross-file reference.  Likewise, do not
        # turn a dangling hyphen into a truncated ID when scanning Markdown.
        if "*" in token or token.endswith("-"):
            continue
        tokens.add(token)
    return tokens


def declaration_identifier(line: str) -> str | None:
    """Return an explicit heading/bullet/table REQ or AC declaration."""
    remainder = line
    heading = HEADING_PREFIX_RE.match(remainder)
    if heading:
        remainder = remainder[heading.end() :]
    else:
        if line.lstrip().startswith("|"):
            cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
            if len(cells) < 2 or not re.search(
                r"要求|验收|说明|描述|条件|requirement|acceptance|description|condition",
                "|".join(cells[1:]),
                re.IGNORECASE,
            ):
                return None
            table = TABLE_ID_CELL_RE.fullmatch(cells[0])
            if not table:
                return None
            token = table.group(1).upper()
            return None if "*" in token or token.endswith("-") else token
        bullet = DECLARATION_PREFIX_RE.match(remainder)
        if not bullet:
            return None
        remainder = remainder[bullet.end() :]
    match = ID_RE.match(remainder)
    if not match:
        return None
    token = match.group(1).upper()
    if "*" in token or token.endswith("-"):
        return None
    return token


def heading_identifier(line: str) -> str | None:
    if not HEADING_PREFIX_RE.match(line):
        return None
    return declaration_identifier(line)


def check_metadata(path: str, text: str, errors: list[str]) -> None:
    text = "\n".join(line for _, line in visible_lines(text))
    required = ["生命周期", "Owner role"]
    authority_labels = ("专业域权威", "专业权威")
    if path.endswith(".prd.md"):
        required.extend(["所属产品模块", "上位产品 PRD"])
    else:
        required.extend(["配对产品 PRD", "上位产品 PRD"])
    for label in required:
        if not metadata_value(text, label):
            fail(errors, "missing-metadata", path, label)
    if not metadata_any(text, authority_labels):
        fail(errors, "missing-metadata", path, "专业域权威 or 专业权威")
    lifecycle = metadata_value(text, "生命周期")
    if lifecycle and not re.search(r"\b(?:proposed|draft|active|superseded|retired)\b", lifecycle):
        fail(errors, "invalid-lifecycle", path, lifecycle)


def check_minimum_topic_content(path: str, text: str, errors: list[str]) -> None:
    visible = "\n".join(line for _, line in visible_lines(text))
    required_groups = {
        "player-scenario": ("玩家", "使用者"),
        "scope": ("范围", "Non-Goals"),
        "normal-path": ("正常", "路径", "流程", "循环", "walkthrough", "链路"),
        "decision": ("选择", "决策", "取舍", "可比较"),
        "failure-recovery": ("失败", "恢复", "阻塞", "blocked"),
        "acceptance": ("验收", "AC-"),
        "evidence-boundary": ("证据", "evidence", "非证明项"),
        "unresolved": ("未决", "假设", "待决", "临时范围"),
    }
    for code, needles in required_groups.items():
        if not any(needle.lower() in visible.lower() for needle in needles):
            fail(errors, f"missing-{code}", path, "topic minimum content marker")


def check_minimum_design_content(path: str, text: str, errors: list[str]) -> None:
    visible = "\n".join(line for _, line in visible_lines(text))
    required_groups = {
        "player-scenario": ("玩家", "使用者"),
        "design-decision": ("选择", "决策", "后果", "取舍"),
        "failure-recovery": ("失败", "恢复", "阻塞", "blocked"),
        "validation-boundary": ("验证", "证据", "evidence"),
    }
    for code, needles in required_groups.items():
        if not any(needle.lower() in visible.lower() for needle in needles):
            fail(errors, f"missing-{code}", path, "design minimum content marker")


def check_requirements(path: str, text: str, errors: list[str]) -> None:
    lines = visible_lines(text)
    prose = "\n".join(ANCHOR_RE.sub("", line) for _, line in lines)
    anchors: dict[str, int] = {}
    for number, line in lines:
        for anchor in ANCHOR_RE.findall(line):
            key = anchor.strip().lower()
            if key in anchors:
                fail(errors, "duplicate-anchor", path, f"{anchor} at lines {anchors[key]} and {number}")
            else:
                anchors[key] = number

    declarations: dict[str, int] = {}
    declaration_kinds: dict[str, str] = {}
    declaration_levels: dict[str, int] = {}
    for number, line in lines:
        identifier = declaration_identifier(line)
        if not identifier:
            continue
        if identifier in declarations:
            fail(errors, "duplicate-id", path, f"{identifier} at lines {declarations[identifier]} and {number}")
        else:
            declarations[identifier] = number
            if HEADING_PREFIX_RE.match(line):
                declaration_kinds[identifier] = "heading"
                heading = HEADING_PREFIX_RE.match(line)
                assert heading is not None
                declaration_levels[identifier] = len(heading.group(0).lstrip().split()[0])
            else:
                declaration_kinds[identifier] = "legacy"
        if declaration_kinds.get(identifier) == "heading" and identifier.lower() not in anchors:
            fail(errors, "missing-anchor", path, f"{identifier} has no <a id=\"{identifier.lower()}\"> anchor")

    # An anchor only becomes a usable local target when a declaration also
    # exists.  The common anchor-before-heading form was already accounted for
    # above; an orphan REQ/AC anchor violates the declaration+anchor contract.
    for anchor, number in anchors.items():
        identifier = anchor.upper()
        if identifier.startswith(("REQ-", "AC-")) and identifier not in declarations and identifier in id_tokens(anchor):
            fail(errors, "anchor-without-declaration", path, f"{anchor} at line {number} has no REQ/AC declaration")

    all_tokens = id_tokens(prose)
    local_tokens = set(declarations)
    links = markdown_links(text)
    linked_fragments: list[tuple[int, str]] = []
    external_fragments: set[tuple[int, str]] = set()
    for number, _raw, target in links:
        link_path, fragment = split_link_target(target)
        if fragment:
            if is_external_link_target(link_path):
                external_fragments.add((number, fragment.upper()))
            else:
                linked_fragments.append((number, fragment.upper()))

    for number, line in lines:
        tokens = id_tokens(ANCHOR_RE.sub("", line)) - local_tokens
        if not tokens:
            continue
        line_fragments = {
            fragment.upper()
            for link_number, fragment in linked_fragments
            if link_number == number
        }
        external_line_fragments = {
            fragment
            for link_number, fragment in external_fragments
            if link_number == number
        }
        for token in sorted(tokens):
            if token in line_fragments:
                continue
            if token in external_line_fragments:
                fail(errors, "external-cross-file-id", path, f"{token} must use a repository-relative Markdown path#{token.lower()} link")
                continue
            if path.endswith(".design.md"):
                fail(errors, "unresolved-cross-file-id", path, f"{token} must use a Markdown path#{token.lower()} link")
            else:
                fail(errors, "unresolved-id-reference", path, f"{token} is not a declared local REQ/AC or path#{token.lower()} reference")

    for index, (number, line) in enumerate(lines):
        identifier = heading_identifier(line)
        if not identifier or declaration_kinds.get(identifier) != "heading":
            continue
        level = declaration_levels[identifier]
        end = len(lines)
        for next_index in range(index + 1, len(lines)):
            next_line = lines[next_index][1]
            next_level_match = re.match(r"^ {0,3}(#{1,6})\s+", next_line)
            if next_level_match and len(next_level_match.group(1)) <= level:
                end = next_index
                break
        block = "\n".join(ANCHOR_RE.sub("", item) for _, item in lines[index + 1 : end])
        if identifier.startswith("REQ-"):
            acceptance_refs = id_tokens(block) & {token for token in all_tokens if token.startswith("AC-")}
            if not acceptance_refs:
                fail(errors, "req-missing-acceptance", path, identifier)
            for reference in acceptance_refs:
                if reference not in local_tokens and not any(fragment == reference for _, fragment in linked_fragments):
                    fail(errors, "unresolved-acceptance", path, f"{identifier} -> {reference}")
        else:
            requirement_refs = id_tokens(block) & {token for token in all_tokens if token.startswith("REQ-")}
            if not requirement_refs:
                fail(errors, "ac-missing-requirement", path, identifier)
            for reference in requirement_refs:
                if reference not in local_tokens and not any(fragment == reference for _, fragment in linked_fragments):
                    fail(errors, "unresolved-requirement", path, f"{identifier} -> {reference}")


def check_document(root: Path, path: str, text: str, errors: list[str]) -> None:
    source = root / path
    if path.endswith("/prd.md"):
        # Canonical module roots retain their existing identity/SC contract;
        # product-doc-governance-check.py is their owner.
        return
    lines = visible_lines(text)
    check_metadata(path, text, errors)
    if path.endswith(".prd.md"):
        check_minimum_topic_content(path, text, errors)
    if path.endswith(".design.md"):
        check_minimum_design_content(path, text, errors)
        expected_prd = path.removesuffix(".design.md") + ".prd.md"
        pair_targets = []
        links = markdown_links(text)
        for _number, _raw, target in links:
            target_path, _fragment = split_link_target(target)
            if target_path:
                resolved = (source.parent / target_path).resolve()
                try:
                    pair_targets.append(resolved.relative_to(root).as_posix())
                except ValueError:
                    pass
        if expected_prd not in pair_targets:
            fail(errors, "missing-paired-prd-link", path, expected_prd)
    links = markdown_links(text)
    for number, line in authority_lines(lines):
        authority_targets = [target for line_number, _raw, target in links if line_number == number]
        if not authority_targets:
            fail(errors, "authority-not-link", path, f"line {number} must use a Markdown path target")
        for target in authority_targets:
            target_path, _fragment = split_link_target(target)
            if "://" in target_path or target_path.startswith(("mailto:", "//")):
                fail(errors, "authority-external-link", path, f"line {number} must resolve inside the repository: {target}")
                continue
            validate_link(root, source, target, text, errors, f"{path}:{number}", "authority-link")
    for number, _raw, target in links:
        validate_link(root, source, target, text, errors, f"{path}:{number}", "markdown-link")
    check_requirements(path, text, errors)


def collect_documents(root: Path, base: str, head: str, worktree: bool) -> tuple[list[ChangedDocument], str | None]:
    changed, worktree_paths = diff_paths(root, base, head, worktree)
    source_base = run_git(root, "merge-base", base, head).strip()
    if not source_base:
        raise ValueError(f"base/head have no merge-base: {base} {head}")
    documents: list[ChangedDocument] = []
    for path, status in sorted(changed.items()):
        if not is_product_doc(path) or status.startswith("D"):
            continue
        old_text = git_text(root, source_base, path)
        new_text = current_text(root, head, path, path in worktree_paths)
        if new_text is None:
            continue
        if status.startswith(("A", "R", "C", "??")) or normalized_for_change(old_text) != normalized_for_change(new_text):
            documents.append(ChangedDocument(path, status, old_text, new_text))
    if not documents:
        return [], "no new or substantive product-document changes in selected range"
    return documents, None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--worktree", action="store_true")
    args = parser.parse_args()
    root = args.repo_root.resolve()
    if args.worktree and not (args.base and args.head):
        print("product-doc-content: error: --worktree requires explicit --base and --head commit OIDs")
        return 2
    try:
        base = resolve_commit(root, args.base, "base")
        head = resolve_commit(root, args.head, "head")
        documents, reason = collect_documents(root, base, head, args.worktree)
    except ValueError as exc:
        print(f"product-doc-content: error: {exc}")
        return 2
    if not documents:
        print(f"product-doc-content: checked 0: reason={reason}")
        return 0
    errors: list[str] = []
    for document in documents:
        check_document(root, document.path, document.new_text, errors)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"product-doc-content: OK (checked {len(documents)} changed/new documents)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
