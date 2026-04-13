#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

ensure_file_contains() {
  local file=$1
  local pattern=$2
  if ! rg -F -q -- "$pattern" "$file"; then
    echo "error: pattern not found: $pattern" >&2
    echo "  file=$file" >&2
    exit 1
  fi
}

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

input_json="$tmpdir/merged-prs.json"
cat >"$input_json" <<'EOF'
[
  {
    "number": 201,
    "title": "Ready reward request",
    "url": "https://github.com/example/oasis7/pull/201",
    "mergedAt": "2026-04-12T10:00:00Z",
    "author": {"login": "builder01"},
    "body": "## Reward Review Intake\n- Request reward review: `yes`\n- Reward Account: `oc:reward:builder01`\n- Evidence / context link: `https://example.com/evidence/201`\n"
  },
  {
    "number": 202,
    "title": "Deferred reward request",
    "url": "https://github.com/example/oasis7/pull/202",
    "mergedAt": "2026-04-12T11:00:00Z",
    "author": {"login": "builder02"},
    "body": "## Reward Review Intake\n- Request reward review: `yes`\n- Reward Account:\n"
  },
  {
    "number": 203,
    "title": "No reward request",
    "url": "https://github.com/example/oasis7/pull/203",
    "mergedAt": "2026-04-12T12:00:00Z",
    "author": {"login": "builder03"},
    "body": "## Summary\n- merged without reward request\n"
  },
  {
    "number": 204,
    "title": "Invalid reward request",
    "url": "https://github.com/example/oasis7/pull/204",
    "mergedAt": "2026-04-12T13:00:00Z",
    "author": {"login": "builder04"},
    "body": "## Reward Review Intake\n- Request reward review: `true`\n- Reward Account: `oc:reward:builder04`\n"
  }
]
EOF

report_json="$tmpdir/report.json"
./scripts/readme-reward-pr-intake-round-scan.py \
  --input-json "$input_json" \
  >"$report_json"

ensure_file_contains "$report_json" '"scanned_prs": 4'
ensure_file_contains "$report_json" '"ready": 1'
ensure_file_contains "$report_json" '"deferred": 1'
ensure_file_contains "$report_json" '"no_reward_review_requested": 1'
ensure_file_contains "$report_json" '"invalid_intake": 1'
ensure_file_contains "$report_json" '"merged_at": "2026-04-12T10:00:00Z"'

summary_txt="$tmpdir/summary.txt"
./scripts/readme-reward-pr-intake-round-scan.py \
  --input-json "$input_json" \
  --format summary \
  >"$summary_txt"

ensure_file_contains "$summary_txt" 'scanned_prs=4'
ensure_file_contains "$summary_txt" 'pr=201 status=ready author=builder01'
ensure_file_contains "$summary_txt" 'pr=204 status=invalid_intake author=builder04'

ledger_md="$tmpdir/ledger.md"
./scripts/readme-reward-pr-intake-round-scan.py \
  --input-json "$input_json" \
  --format ledger-md \
  >"$ledger_md"

ensure_file_contains "$ledger_md" 'LTRL-PR-201'
ensure_file_contains "$ledger_md" 'LTRL-PR-202'
ensure_file_contains "$ledger_md" 'oc:reward:builder01'
ensure_file_contains "$ledger_md" 'deferred'

report_like_json="$tmpdir/report-like.json"
cat >"$report_like_json" <<'EOF'
{
  "entries": [
    {
      "pr_number": 201,
      "import_status": "ready",
      "pr_url": "https://github.com/example/oasis7/pull/201"
    }
  ]
}
EOF

set +e
invalid_input_output=$(
  ./scripts/readme-reward-pr-intake-round-scan.py \
    --input-json "$report_like_json" \
    --format summary \
    2>&1
)
invalid_input_code=$?
set -e

if [[ "$invalid_input_code" -eq 0 ]]; then
  echo "error: expected report-like input json to fail" >&2
  exit 1
fi

if ! grep -q 'missing required key: body' <<<"$invalid_input_output"; then
  echo "error: expected missing body failure output" >&2
  echo "$invalid_input_output" >&2
  exit 1
fi

echo "readme reward PR intake round scan smoke checks passed"
