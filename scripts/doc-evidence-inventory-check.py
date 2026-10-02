#!/usr/bin/env python3
"""Check v3 evidence shards with the shared corpus parser and frozen policy."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import document_corpus as dc
import document_evidence_policy as evidence_policy


BASELINE = Path(__file__).resolve().parent / "fixtures/document-corpus-v3/legacy/inventory.json"


def check(root: Path, inventory_path: Path | None = None, revision: str | None = None) -> list[dc.Diagnostic]:
    del inventory_path  # v1 path is a migration input only; v3 has fixed roots.
    try:
        view: dc.CorpusView = dc.GitCorpusView(root, revision) if revision else dc.WorktreeCorpusView(root)
        model = dc.load_corpus(view)
        errors = dc.validate_evidence(view, model)
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


def _format(item: dc.Diagnostic) -> str:
    fields = [item.code]
    if item.source_path:
        fields.append(f"source={item.source_path}")
    if item.record_path:
        fields.append(f"record={item.record_path}")
    if item.detail:
        fields.append(item.detail)
    if item.repair_command:
        fields.append(f"repair: {item.repair_command}")
    return ": ".join(fields)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--revision")
    group.add_argument("--worktree", action="store_true")
    parser.add_argument("--print-generated", action="store_true")
    args = parser.parse_args()
    if args.print_generated:
        print("doc-evidence-inventory-check: --print-generated is retired; use document-corpus-inventory.py export. Shell redirection can truncate a target before this command starts.", file=sys.stderr)
        return 2
    errors = check(args.repo_root.resolve(), revision=args.revision)
    if errors:
        print("doc-evidence-inventory-check: FAIL", file=sys.stderr)
        for item in errors:
            print(json.dumps(item.as_dict(), ensure_ascii=False, sort_keys=True), file=sys.stderr)
        return 1
    print("doc-evidence-inventory-check: OK")
    snapshot_view: dc.CorpusView = dc.GitCorpusView(args.repo_root.resolve(), args.revision) if args.revision else dc.WorktreeCorpusView(args.repo_root.resolve())
    print(json.dumps({"mode": "revision" if args.revision else "worktree", **dc.snapshot_info(snapshot_view)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
