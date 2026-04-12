#!/usr/bin/env python3
"""Parse GitHub PR reward intake blocks into ledger-ready fields."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

FIELD_ALIASES = {
    "request reward review": "reward_review_request",
    "reward account": "reward_account",
    "evidence / context link": "evidence_context_link",
    "notes": "notes",
}

LEDGER_HEADERS = [
    "Ledger ID",
    "Contributor",
    "Public Handle / GitHub",
    "Reward Account",
    "Source Type",
    "Source Link",
    "Contribution Type",
    "Base Score",
    "Quality Modifier",
    "Total Score",
    "Recommended Band",
    "Duplicate Check",
    "Reviewer",
    "Review Status",
    "Producer Decision",
    "Approval ID",
    "Actual Amount",
    "Distribution Ref",
    "Distribution Date",
    "Notes",
]


def fail(msg: str) -> None:
    print(f"error: {msg}", file=sys.stderr)
    raise SystemExit(1)


def require_cmd(name: str) -> None:
    if shutil.which(name) is None:
        fail(f"missing required command: {name}")


def run_cmd(args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, text=True, capture_output=True, check=True)


def normalize_value(raw: str) -> str:
    value = raw.strip()
    if value.startswith("`") and value.endswith("`") and len(value) >= 2:
        value = value[1:-1].strip()
    return value


def parse_reward_review_requested(raw: str) -> bool:
    normalized = normalize_value(raw).lower()
    return normalized == "yes"


def parse_intake_block(body: str) -> dict[str, Any]:
    lines = body.splitlines()
    start_index: int | None = None

    for idx, line in enumerate(lines):
        if line.strip() == "## Reward Review Intake":
            start_index = idx + 1
            break

    if start_index is None:
        return {
            "intake_present": False,
            "raw_section": "",
            "fields": {},
        }

    section_lines: list[str] = []
    for line in lines[start_index:]:
        if line.startswith("## "):
            break
        section_lines.append(line)

    fields: dict[str, str] = {}
    current_key: str | None = None

    for raw_line in section_lines:
        line = raw_line.rstrip()
        match = re.match(r"^-\s+([^:]+):\s*(.*)$", line)
        if match:
            label = match.group(1).strip().lower()
            mapped = FIELD_ALIASES.get(label)
            if mapped is None:
                current_key = None
                continue
            fields[mapped] = normalize_value(match.group(2))
            current_key = mapped
            continue

        stripped = line.strip()
        if current_key and stripped and not stripped.startswith("- "):
            previous = fields.get(current_key, "")
            merged = f"{previous}\n{normalize_value(stripped)}".strip()
            fields[current_key] = merged
            continue

        current_key = None

    return {
        "intake_present": True,
        "raw_section": "\n".join(section_lines).strip(),
        "fields": fields,
    }


def fetch_pr_metadata(pr_number: int, repo: str) -> dict[str, Any]:
    require_cmd("gh")
    cmd = ["gh", "pr", "view", str(pr_number), "--json", "number,title,body,url,author"]
    if repo:
        cmd.extend(["--repo", repo])
    try:
        proc = run_cmd(cmd)
    except subprocess.CalledProcessError as exc:
        detail = (exc.stderr or exc.stdout or "").strip()
        if detail:
            fail(f"`gh pr view` failed for PR {pr_number}: {detail}")
        fail(f"`gh pr view` failed for PR {pr_number}")
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        fail(f"failed to parse JSON from gh pr view: {exc}")
    return payload


def md_escape(value: str) -> str:
    escaped = value.replace("|", "\\|").replace("\n", "<br>")
    return escaped


def build_result(
    *,
    body: str,
    source_link: str,
    title: str,
    contributor: str,
    public_handle: str,
    pr_number: int | None,
    pr_url: str,
    ledger_id: str,
    contribution_type: str,
) -> dict[str, Any]:
    parsed = parse_intake_block(body)
    fields = parsed["fields"]
    intake_present = bool(parsed["intake_present"])
    request_raw = fields.get("reward_review_request", "")
    request_yes = parse_reward_review_requested(request_raw)
    validation_errors: list[str] = []
    missing_fields: list[str] = []

    if not intake_present:
        import_status = "no_reward_review_requested"
    elif not request_yes:
        import_status = "invalid_intake"
        validation_errors.append(
            "Reward Review Intake block is present but `Request reward review` is not explicit `yes`."
        )
    else:
        if not fields.get("reward_account", ""):
            missing_fields.append("reward_account")
        import_status = "ready" if not missing_fields else "deferred"

    row_source_link = pr_url or source_link
    notes_parts: list[str] = []
    if title:
        notes_parts.append(f"pr_title={title}")
    evidence_context_link = fields.get("evidence_context_link", "")
    if evidence_context_link:
        notes_parts.append(f"evidence_context_link={evidence_context_link}")
    intake_notes = fields.get("notes", "")
    if intake_notes:
        notes_parts.append(f"intake_notes={intake_notes}")
    if validation_errors:
        notes_parts.extend(f"validation_error={error}" for error in validation_errors)
    if missing_fields:
        notes_parts.append("missing_fields=" + ",".join(missing_fields))
    ledger_notes = "; ".join(notes_parts)

    ledger_row: dict[str, str] | None
    if import_status in {"no_reward_review_requested", "invalid_intake"}:
        ledger_row = None
    else:
        review_status = "draft" if import_status == "ready" else "deferred"
        ledger_row = {
            "ledger_id": ledger_id,
            "contributor": contributor,
            "public_handle_or_github": public_handle,
            "reward_account": fields.get("reward_account", ""),
            "source_type": "PR",
            "source_link": row_source_link,
            "contribution_type": contribution_type,
            "review_status": review_status,
            "notes": ledger_notes,
        }

    result = {
        "import_status": import_status,
        "intake_present": intake_present,
        "reward_review_requested": request_yes,
        "validation_errors": validation_errors,
        "missing_fields": missing_fields,
        "pr_number": pr_number,
        "pr_url": pr_url,
        "source_link": source_link,
        "title": title,
        "contributor": contributor,
        "public_handle_or_github": public_handle,
        "reward_account": fields.get("reward_account", ""),
        "evidence_context_link": evidence_context_link,
        "notes": intake_notes,
        "ledger_row": ledger_row,
    }
    return result


def render_ledger_row(result: dict[str, Any]) -> str:
    status = result["import_status"]
    if result["ledger_row"] is None:
        return f"# no ledger row emitted ({status})"

    row = result["ledger_row"]
    assert row is not None

    values = [
        row["ledger_id"],
        row["contributor"],
        row["public_handle_or_github"],
        row["reward_account"],
        row["source_type"],
        row["source_link"],
        row["contribution_type"],
        "",
        "",
        "",
        "",
        "",
        "",
        row["review_status"],
        "",
        "",
        "",
        "",
        "",
        row["notes"],
    ]
    if len(values) != len(LEDGER_HEADERS):
        fail("internal error: ledger row length mismatch")
    return "| " + " | ".join(md_escape(value) for value in values) + " |"


def render_summary(result: dict[str, Any]) -> str:
    status = result["import_status"]
    if status == "no_reward_review_requested":
        return "status=no_reward_review_requested"
    if status == "ready":
        return f"status=ready reward_account={result['reward_account']}"
    missing = ",".join(result["missing_fields"])
    if missing:
        return f"status={status} missing_fields={missing}"
    return f"status={status}"


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
