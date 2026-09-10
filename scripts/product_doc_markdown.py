#!/usr/bin/env python3
"""Small shared Markdown link-node adapter for product-document gates.

The gates need actual CommonMark link nodes, rather than a second regular
expression approximation.  ``markdown-it-py`` is pinned in
``scripts/doc-governance-requirements.txt`` and uses its CommonMark preset;
GFM extensions are intentionally not required for this contract.
"""

from __future__ import annotations

from dataclasses import dataclass

try:
    from markdown_it import MarkdownIt
except ModuleNotFoundError as exc:  # pragma: no cover - exercised by setup checks
    raise RuntimeError(
        "product-document Markdown parsing requires markdown-it-py; "
        "install scripts/doc-governance-requirements.txt"
    ) from exc


_MARKDOWN = MarkdownIt("commonmark")


@dataclass(frozen=True)
class MarkdownLink:
    """A genuine Markdown link node and its source block line."""

    line: int
    target: str


def parse_markdown_links(text: str) -> tuple[MarkdownLink, ...]:
    """Return clickable Markdown link nodes, excluding images and code nodes.

    ``token.map`` is the source block's zero-based line span.  A multi-line
    destination is attributed to its first source line; the gates only use the
    line for diagnostics and link-node association.
    """
    links: list[MarkdownLink] = []
    source_lines = text.splitlines()
    for token in _MARKDOWN.parse(text):
        if token.type != "inline" or not token.children or not token.map:
            continue
        start, end = token.map
        line = start + 1
        search_line = start
        search_offset = 0
        for child in token.children:
            if child.type != "link_open":
                continue
            attrs = dict(child.attrs or ())
            target = attrs.get("href")
            if target:
                for source_index in range(search_line, min(end, len(source_lines))):
                    offset = search_offset if source_index == search_line else 0
                    position = source_lines[source_index].find(target, offset)
                    if position < 0:
                        continue
                    line = source_index + 1
                    search_line = source_index
                    search_offset = position + len(target)
                    break
                links.append(MarkdownLink(line=line, target=target))
    return tuple(links)
