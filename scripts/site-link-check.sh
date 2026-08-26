#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SITE_ROOT="${REPO_ROOT}/site"

if [[ ! -d "${SITE_ROOT}" ]]; then
  echo "error: site root not found: ${SITE_ROOT}" >&2
  exit 1
fi

python3 - "${SITE_ROOT}" <<'PY'
from __future__ import annotations

import os
import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

site_root = Path(sys.argv[1])
attr_re = re.compile(r'(href|src)="([^"]+)"')
failures: list[tuple[Path, str, Path, str]] = []


class FragmentCollector(HTMLParser):
    """Collect anchors from HTML and SVG-like assets without third-party parsers."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.anchors: set[str] = set()

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = {name.lower(): value for name, value in attrs}
        element_id = values.get("id")
        if element_id:
            self.anchors.add(element_id)
        if tag.lower() in {"a", "area"} and values.get("name"):
            self.anchors.add(values["name"] or "")

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.handle_starttag(tag, attrs)


def collect_fragments(target: Path) -> set[str]:
    try:
        contents = target.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return set()
    parser = FragmentCollector()
    try:
        parser.feed(contents)
        parser.close()
    except Exception:
        # A malformed asset cannot prove that its requested fragment exists.
        return set()
    return parser.anchors


def resolve_target(html_file: Path, path_part: str) -> Path:
    decoded_path = unquote(path_part)
    if not decoded_path:
        return html_file
    if decoded_path.startswith("/"):
        return Path(os.path.normpath(os.path.abspath(site_root / decoded_path.lstrip("/"))))
    return Path(os.path.normpath(os.path.abspath(html_file.parent / decoded_path)))

for html_file in sorted(site_root.rglob("*.html")):
    text = html_file.read_text(encoding="utf-8")
    for _attr, ref in attr_re.findall(text):
        if not ref or ref.startswith("//"):
            continue
        parsed = urlsplit(ref)
        if parsed.scheme or parsed.netloc:
            continue
        target = resolve_target(html_file, parsed.path)
        if not target.exists():
            failures.append((html_file, ref, target, "target does not exist"))
            continue

        fragment = unquote(parsed.fragment)
        if not fragment:
            continue

        fragment_target = target
        if fragment_target.is_dir():
            fragment_target = fragment_target / "index.html"
        if not fragment_target.exists():
            failures.append((html_file, ref, fragment_target, "fragment target does not exist"))
            continue
        if fragment not in collect_fragments(fragment_target):
            failures.append((html_file, ref, fragment_target, f"fragment #{fragment} does not exist"))

for html_file, ref, target, reason in failures:
    print(f"error: broken local reference in {html_file}: {ref} -> {target} ({reason})", file=sys.stderr)

if failures:
    print(
        f"error: site link check failed with {len(failures)} broken local reference(s)",
        file=sys.stderr,
    )
    raise SystemExit(1)

print("ok: site local href/src references are valid")
PY
