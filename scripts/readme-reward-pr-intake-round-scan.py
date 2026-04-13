#!/usr/bin/env python3
"""Batch-scan merged PR reward intake blocks for one reward review round."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from readme_reward_pr_intake_lib import (
    build_result_from_pr_payload,
    fail,
    render_ledger_row,
    require_cmd,
    run_cmd,
)


def load_input_entries(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        fail(f"--input-json not found: {path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"failed to parse --input-json: {exc}")

    if isinstance(payload, list):
        entries = payload
    elif isinstance(payload, dict) and isinstance(payload.get("entries"), list):
        entries = payload["entries"]
    else:
        fail("--input-json must be a JSON array or an object with an `entries` array")

    normalized: list[dict[str, Any]] = []
    for idx, entry in enumerate(entries, start=1):
        if not isinstance(entry, dict):
            fail(f"--input-json entry #{idx} is not an object")
        if "body" not in entry:
            fail(f"--input-json entry #{idx} is missing required key: body")
        if not isinstance(entry.get("body"), str):
            fail(f"--input-json entry #{idx} has non-string body")
        normalized.append(entry)
    return normalized


def build_search_query(merged_after: str, merged_before: str, extra_search: str) -> str:
    terms: list[str] = []
    if merged_after:
        terms.append(f"merged:>={merged_after}")
    if merged_before:
        terms.append(f"merged:<={merged_before}")
    if extra_search:
        terms.append(extra_search.strip())
    return " ".join(term for term in terms if term)


def fetch_merged_pr_entries(
    *,
    repo: str,
    merged_after: str,
    merged_before: str,
    extra_search: str,
    limit: int,
) -> list[dict[str, Any]]:
    require_cmd("gh")
    cmd = [
        "gh",
        "pr",
        "list",
        "--state",
        "merged",
        "--limit",
        str(limit),
        "--json",
        "number,title,body,url,author,mergedAt",
    ]
    if repo:
        cmd.extend(["--repo", repo])
    search_query = build_search_query(merged_after, merged_before, extra_search)
    if search_query:
        cmd.extend(["--search", search_query])
    try:
        proc = run_cmd(cmd)
    except Exception as exc:
        detail = ""
        if hasattr(exc, "stderr") or hasattr(exc, "stdout"):
            detail = str(getattr(exc, "stderr", "") or getattr(exc, "stdout", "")).strip()
        if detail:
            fail(f"`gh pr list` failed: {detail}")
        fail(f"`gh pr list` failed: {exc}")
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        fail(f"failed to parse JSON from gh pr list: {exc}")
    if not isinstance(payload, list):
        fail("unexpected gh pr list payload: expected array")
    return [entry for entry in payload if isinstance(entry, dict)]


def make_ledger_id(prefix: str, pr_number: int | None, index: int) -> str:
    if pr_number:
        return f"{prefix}-{pr_number}"
    return f"{prefix}-{index:03d}"


def build_report(
    entries: list[dict[str, Any]],
    *,
    ledger_prefix: str,
    contribution_type: str,
    repo: str,
    merged_after: str,
    merged_before: str,
    search_query: str,
    source_kind: str,
) -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    status_counts = {
        "ready": 0,
        "deferred": 0,
        "no_reward_review_requested": 0,
        "invalid_intake": 0,
    }

    for index, entry in enumerate(entries, start=1):
        pr_number = int(entry.get("number", 0) or 0) or None
        ledger_id = make_ledger_id(ledger_prefix, pr_number, index)
        result = build_result_from_pr_payload(
            entry,
            ledger_id=ledger_id,
            contribution_type=contribution_type,
        )
        status = result["import_status"]
        status_counts[status] = status_counts.get(status, 0) + 1
        result["merged_at"] = str(entry.get("mergedAt", "") or "")
        results.append(result)

    return {
        "source_kind": source_kind,
        "repo": repo,
        "merged_after": merged_after,
        "merged_before": merged_before,
        "search_query": search_query,
        "ledger_prefix": ledger_prefix,
        "contribution_type": contribution_type,
        "scanned_prs": len(results),
        "status_counts": status_counts,
        "entries": results,
    }


def render_scan_summary(report: dict[str, Any]) -> str:
    counts = report["status_counts"]
    lines = [
        f"scanned_prs={report['scanned_prs']}",
        (
            "status_counts="
            f"ready:{counts.get('ready', 0)},"
            f"deferred:{counts.get('deferred', 0)},"
            f"no_reward_review_requested:{counts.get('no_reward_review_requested', 0)},"
            f"invalid_intake:{counts.get('invalid_intake', 0)}"
        ),
    ]
    for entry in report["entries"]:
        lines.append(
            "pr="
            f"{entry.get('pr_number') or 'unknown'} "
            f"status={entry['import_status']} "
            f"author={entry.get('public_handle_or_github', '')} "
            f"url={entry.get('pr_url', '')}"
        )
    return "\n".join(lines)


def render_scan_ledger_rows(report: dict[str, Any]) -> str:
    rows = [
        render_ledger_row(entry)
        for entry in report["entries"]
        if entry.get("ledger_row") is not None
    ]
    if not rows:
        return "# no ledger rows emitted"
    return "\n".join(rows)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    source_group = parser.add_mutually_exclusive_group(required=True)
    source_group.add_argument(
        "--input-json",
        default="",
        help="Offline JSON array or object with `entries` used for smoke/tests",
    )
    source_group.add_argument(
        "--use-gh",
        action="store_true",
        help="Scan merged PRs via `gh pr list`",
    )
    parser.add_argument("--repo", default="", help="owner/repo for `gh pr list`")
    parser.add_argument("--merged-after", default="", help="Merged-at lower bound (inclusive)")
    parser.add_argument("--merged-before", default="", help="Merged-at upper bound (inclusive)")
    parser.add_argument("--search", default="", help="Extra gh search query fragment")
    parser.add_argument("--limit", type=int, default=100, help="Max PRs to scan")
    parser.add_argument(
        "--ledger-prefix",
        default="LTRL-PR",
        help="Ledger ID prefix for emitted candidate rows",
    )
    parser.add_argument(
        "--contribution-type",
        default="C-03",
        help="Contribution type to place in emitted ledger rows (default: C-03)",
    )
    parser.add_argument(
        "--format",
        choices=("json", "summary", "ledger-md"),
        default="json",
        help="Output format",
    )
    args = parser.parse_args()

    if args.limit <= 0:
        fail("--limit must be > 0")

    if args.use_gh and not (args.merged_after or args.merged_before or args.search):
        fail("live merged PR scan requires at least one filter: --merged-after, --merged-before, or --search")

    if args.input_json:
        entries = load_input_entries(Path(args.input_json))
        source_kind = "input_json"
    else:
        entries = fetch_merged_pr_entries(
            repo=args.repo,
            merged_after=args.merged_after,
            merged_before=args.merged_before,
            extra_search=args.search,
            limit=args.limit,
        )
        source_kind = "gh_pr_list"

    search_query = build_search_query(args.merged_after, args.merged_before, args.search)
    report = build_report(
        entries,
        ledger_prefix=args.ledger_prefix,
        contribution_type=args.contribution_type,
        repo=args.repo,
        merged_after=args.merged_after,
        merged_before=args.merged_before,
        search_query=search_query,
        source_kind=source_kind,
    )

    if args.format == "json":
        print(json.dumps(report, ensure_ascii=True, indent=2))
    elif args.format == "summary":
        print(render_scan_summary(report))
    else:
        print(render_scan_ledger_rows(report))


if __name__ == "__main__":
    main()
