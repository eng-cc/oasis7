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

ready_body="$tmpdir/ready.md"
cat >"$ready_body" <<'EOF'
## Summary
- reward import smoke

## Validation
- smoke only

## Reward Review Intake
Delete this entire section if you do not want this PR to be considered in the early contributor reward review.

- Request reward review: `yes`
- Oasis ID: `oasis-builder-01`
- Reward Account: `awt:reward:builder01`
- Evidence / context link: `https://example.com/evidence/123`
- Notes: `Merged after reviewer feedback`
EOF

ready_json="$tmpdir/ready.json"
./scripts/readme-reward-pr-intake-import.py \
  --body-file "$ready_body" \
  --source-link "https://github.com/example/oasis7/pull/123" \
  --title "Improve reward intake parser" \
  --public-handle "builder01" \
  --contributor "@builder01" \
  --ledger-id "LTRL-PR-001" \
  >"$ready_json"

ensure_file_contains "$ready_json" '"import_status": "ready"'
ensure_file_contains "$ready_json" '"oasis_id": "oasis-builder-01"'
ensure_file_contains "$ready_json" '"reward_account": "awt:reward:builder01"'
ensure_file_contains "$ready_json" '"source_link": "https://github.com/example/oasis7/pull/123"'

ready_row="$tmpdir/ready-row.md"
./scripts/readme-reward-pr-intake-import.py \
  --body-file "$ready_body" \
  --source-link "https://github.com/example/oasis7/pull/123" \
  --title "Improve reward intake parser" \
  --public-handle "builder01" \
  --contributor "@builder01" \
  --ledger-id "LTRL-PR-001" \
  --format ledger-md \
  >"$ready_row"

ensure_file_contains "$ready_row" 'LTRL-PR-001'
ensure_file_contains "$ready_row" 'oasis-builder-01'
ensure_file_contains "$ready_row" 'awt:reward:builder01'
ensure_file_contains "$ready_row" 'draft'

no_request_body="$tmpdir/no-request.md"
cat >"$no_request_body" <<'EOF'
## Summary
- no reward request

## Validation
- none
EOF

no_request_summary="$tmpdir/no-request.txt"
./scripts/readme-reward-pr-intake-import.py \
  --body-file "$no_request_body" \
  --source-link "https://github.com/example/oasis7/pull/124" \
  --format summary \
  >"$no_request_summary"

ensure_file_contains "$no_request_summary" 'status=no_reward_review_requested'

invalid_body="$tmpdir/invalid.md"
cat >"$invalid_body" <<'EOF'
## Reward Review Intake
- Request reward review: `true`
- Oasis ID: `oasis-builder-03`
- Reward Account: `awt:reward:builder03`
EOF

invalid_summary="$tmpdir/invalid-summary.txt"
./scripts/readme-reward-pr-intake-import.py \
  --body-file "$invalid_body" \
  --source-link "https://github.com/example/oasis7/pull/125" \
  --format summary \
  >"$invalid_summary"

ensure_file_contains "$invalid_summary" 'status=invalid_intake'

invalid_row="$tmpdir/invalid-row.txt"
./scripts/readme-reward-pr-intake-import.py \
  --body-file "$invalid_body" \
  --source-link "https://github.com/example/oasis7/pull/125" \
  --format ledger-md \
  >"$invalid_row"

ensure_file_contains "$invalid_row" '# no ledger row emitted (invalid_intake)'

set +e
missing_link_output=$(
  ./scripts/readme-reward-pr-intake-import.py \
    --body-file "$invalid_body" \
    --format summary \
    2>&1
)
missing_link_code=$?
set -e

if [[ "$missing_link_code" -eq 0 ]]; then
  echo "error: expected body-file import without source link to fail" >&2
  exit 1
fi

if ! grep -q -- '--source-link is required with --body-file' <<<"$missing_link_output"; then
  echo "error: expected missing source-link failure output" >&2
  echo "$missing_link_output" >&2
  exit 1
fi

missing_body="$tmpdir/missing.md"
cat >"$missing_body" <<'EOF'
## Summary
- missing reward account

## Validation
- none

## Reward Review Intake
Delete this entire section if you do not want this PR to be considered in the early contributor reward review.

- Request reward review: `yes`
- Oasis ID: `oasis-builder-02`
- Reward Account:
- Evidence / context link: `https://example.com/evidence/456`
EOF

set +e
missing_output=$(
  ./scripts/readme-reward-pr-intake-import.py \
    --body-file "$missing_body" \
    --source-link "https://github.com/example/oasis7/pull/126" \
    --require-ready \
    2>&1
)
missing_code=$?
set -e

if [[ "$missing_code" -eq 0 ]]; then
  echo "error: expected missing-field import to fail with --require-ready" >&2
  exit 1
fi

if ! grep -q 'reward intake is not ready: deferred' <<<"$missing_output"; then
  echo "error: expected deferred failure output" >&2
  echo "$missing_output" >&2
  exit 1
fi

echo "readme reward PR intake import smoke checks passed"
