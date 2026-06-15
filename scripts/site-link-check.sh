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
from pathlib import Path

site_root = Path(sys.argv[1])
attr_re = re.compile(r'(href|src)="([^"]+)"')
skip_prefixes = ("#", "http:", "https:", "mailto:", "javascript:", "tel:")
failures: list[tuple[Path, str, Path]] = []

for html_file in sorted(site_root.rglob("*.html")):
    text = html_file.read_text(encoding="utf-8")
    for _attr, ref in attr_re.findall(text):
        if not ref or ref.startswith(skip_prefixes):
            continue
        clean = ref.split("#", 1)[0].split("?", 1)[0]
        if not clean:
            continue
        target = Path(os.path.normpath(os.path.abspath(html_file.parent / clean)))
        if not target.exists():
            failures.append((html_file, ref, target))

for html_file, ref, target in failures:
    print(f"error: broken local reference in {html_file}: {ref} -> {target}", file=sys.stderr)

if failures:
    print(
        f"error: site link check failed with {len(failures)} broken local reference(s)",
        file=sys.stderr,
    )
    raise SystemExit(1)

print("ok: site local href/src references are valid")
PY
