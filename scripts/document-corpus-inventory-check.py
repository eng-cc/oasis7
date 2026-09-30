#!/usr/bin/env python3
"""Check complete document corpus v3 coverage and review bindings."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import document_corpus as dc
import document_evidence_policy as evidence_policy


BASELINE = Path(__file__).resolve().parent / "fixtures/document-corpus-v3/legacy/inventory.json"


def check(root: Path, revision: str | None = None) -> list[dc.Diagnostic]:
    try:
        view: dc.CorpusView = dc.GitCorpusView(root, revision) if revision else dc.WorktreeCorpusView(root)
        model = dc.load_corpus(view)
        errors = dc.validate_corpus(view, model)
        errors.extend(dc.validate_evidence(view, model))
        try:
            baseline = BASELINE.read_bytes()
        except OSError as exc:
            errors.append(dc.Diagnostic("frozen-baseline-invalid", detail=f"cannot read immutable evidence fixture: {exc}"))
        else:
            errors.extend(evidence_policy.validate_evidence_policy(view, model, baseline))
        if isinstance(view, dc.WorktreeCorpusView):
            errors.extend(view.verify_unchanged())
        return errors
    except dc.CorpusError as exc:
        return [exc.diagnostic]


def format_diagnostic(item: dc.Diagnostic) -> str:
    location = ""
    if item.source_path:
        location += f" source={item.source_path}"
    if item.record_path:
        location += f" record={item.record_path}"
    detail = f": {item.detail}" if item.detail else ""
    repair = f"; repair: {item.repair_command}" if item.repair_command else ""
    return f"{item.code}{location}{detail}{repair}"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--revision")
    group.add_argument("--worktree", action="store_true")
    parser.add_argument("--print-generated", action="store_true")
    args = parser.parse_args()
    if args.print_generated:
        print("document-corpus-inventory-check: --print-generated is retired; use document-corpus-inventory.py export. Shell redirection can truncate a target before this command starts.", file=sys.stderr)
        return 2
    errors = check(args.repo_root.resolve(), args.revision)
    if errors:
        print("document-corpus-inventory-check: FAIL")
        for item in errors:
            print(json.dumps(item.as_dict(), ensure_ascii=False, sort_keys=True))
        return 1
    print("document-corpus-inventory-check: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
