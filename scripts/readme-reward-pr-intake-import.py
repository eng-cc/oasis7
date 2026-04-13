#!/usr/bin/env python3
"""Parse GitHub PR reward intake blocks into ledger-ready fields."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from readme_reward_pr_intake_lib import (
    build_result,
    fail,
    fetch_pr_metadata,
    render_ledger_row,
    render_summary,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    source_group = parser.add_mutually_exclusive_group(required=True)
    source_group.add_argument("--pr", type=int, default=0, help="GitHub PR number")
    source_group.add_argument("--body-file", default="", help="Path to a PR body file")
    parser.add_argument("--repo", default="", help="owner/repo for `gh pr view`")
    parser.add_argument(
        "--source-link",
        default="",
        help="Source link for emitted rows; required with --body-file",
    )
    parser.add_argument("--title", default="", help="Fallback title for --body-file")
    parser.add_argument(
        "--public-handle",
        default="",
        help="Fallback Public Handle / GitHub field for --body-file",
    )
    parser.add_argument(
        "--contributor",
        default="",
        help="Fallback Contributor field for --body-file",
    )
    parser.add_argument("--ledger-id", default="", help="Optional ledger id for row rendering")
    parser.add_argument(
        "--contribution-type",
        default="C-03",
        help="Contribution type to place in emitted ledger rows (default: C-03)",
    )
    parser.add_argument(
        "--format",
        choices=("json", "ledger-md", "summary"),
        default="json",
        help="Output format",
    )
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="Fail unless the intake block is import-ready",
    )
    args = parser.parse_args()

    pr_number: int | None = None
    pr_url = ""
    body = ""
    title = args.title
    public_handle = args.public_handle
    contributor = args.contributor
    source_link = args.source_link

    if args.pr:
        payload = fetch_pr_metadata(args.pr, args.repo)
        pr_number = int(payload.get("number", 0)) or args.pr
        pr_url = str(payload.get("url", "") or "")
        body = str(payload.get("body", "") or "")
        title = title or str(payload.get("title", "") or "")
        author = payload.get("author")
        if isinstance(author, dict):
            public_handle = public_handle or str(author.get("login", "") or "")
        contributor = contributor or (f"@{public_handle}" if public_handle else "")
        source_link = source_link or pr_url
    else:
        body_path = Path(args.body_file)
        if not body_path.is_file():
            fail(f"--body-file not found: {body_path}")
        body = body_path.read_text(encoding="utf-8")
        if not source_link:
            fail("--source-link is required with --body-file")

    result = build_result(
        body=body,
        source_link=source_link,
        title=title,
        contributor=contributor,
        public_handle=public_handle,
        pr_number=pr_number,
        pr_url=pr_url,
        ledger_id=args.ledger_id,
        contribution_type=args.contribution_type,
    )

    if args.require_ready and result["import_status"] != "ready":
        msg = f"reward intake is not ready: {result['import_status']}"
        missing = result["missing_fields"]
        if missing:
            msg += f" (missing: {','.join(missing)})"
        fail(msg)

    if args.format == "json":
        print(json.dumps(result, ensure_ascii=True, indent=2))
    elif args.format == "ledger-md":
        print(render_ledger_row(result))
    else:
        print(render_summary(result))


if __name__ == "__main__":
    main()
