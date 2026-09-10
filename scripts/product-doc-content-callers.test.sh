#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

base_oid="$(git rev-parse HEAD^)"
head_oid="$(git rev-parse HEAD)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; OASIS7_PRODUCT_DOC_BASE="$base_oid" ./scripts/doc-governance-check.sh) >"$tmp_dir/partial.out" 2>&1; then
  echo "product-doc-content-callers.test: partial local base/head unexpectedly passed" >&2
  exit 1
fi
grep -Fq "explicit base/head must be supplied together" "$tmp_dir/partial.out"

if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; OASIS7_PRODUCT_DOC_HEAD="$head_oid" ./scripts/doc-governance-check.sh) >"$tmp_dir/partial-head.out" 2>&1; then
  echo "product-doc-content-callers.test: partial head/base unexpectedly passed" >&2
  exit 1
fi
grep -Fq "explicit base/head must be supplied together" "$tmp_dir/partial-head.out"

if python3 ./scripts/product-doc-content-check.py --base HEAD --head "$head_oid" >"$tmp_dir/label.out" 2>&1; then
  echo "product-doc-content-callers.test: symbolic checker identity unexpectedly passed" >&2
  exit 1
fi
grep -Fq "must be a full 40-character commit OID" "$tmp_dir/label.out"

range_function="$(sed -n '/^product_doc_range()/,/^run_product_doc_governance_check()/p' ./scripts/ci-tests.sh | sed '$d')"
eval "$range_function"
printf '{"pull_request":{"base":{"sha":"%s"},"head":{"sha":"%s"}}}\n' "$base_oid" "$head_oid" >"$tmp_dir/pull_request.json"
valid_range="$(unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; GITHUB_EVENT_PATH="$tmp_dir/pull_request.json" GITHUB_EVENT_NAME=pull_request GITHUB_SHA="$head_oid" product_doc_range)"
[[ "$valid_range" == "$base_oid
$head_oid" ]]

printf '{}\n' >"$tmp_dir/empty.json"
if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; GITHUB_EVENT_PATH="$tmp_dir/empty.json" GITHUB_EVENT_NAME=unknown GITHUB_SHA="$head_oid" product_doc_range) >"$tmp_dir/event.out" 2>&1; then
  echo "product-doc-content-callers.test: malformed CI event unexpectedly fell back" >&2
  exit 1
fi
grep -Fq "unsupported CI event range" "$tmp_dir/event.out"

printf '{"before":"%s"}\n' "$base_oid" >"$tmp_dir/push-missing-head.json"
if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; GITHUB_EVENT_PATH="$tmp_dir/push-missing-head.json" GITHUB_EVENT_NAME=push GITHUB_SHA="$head_oid" product_doc_range) >"$tmp_dir/push.out" 2>&1; then
  echo "product-doc-content-callers.test: incomplete push event unexpectedly fell back" >&2
  exit 1
fi
grep -Fq "did not provide both base/head OIDs" "$tmp_dir/push.out"

printf '{}\n' >"$tmp_dir/schedule.json"
if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; GITHUB_EVENT_PATH="$tmp_dir/schedule.json" GITHUB_EVENT_NAME=schedule GITHUB_SHA="$head_oid" product_doc_range) >"$tmp_dir/schedule.out" 2>&1; then
  echo "product-doc-content-callers.test: schedule event unexpectedly supplied a range" >&2
  exit 1
fi
grep -Fq "unsupported CI event range: schedule" "$tmp_dir/schedule.out"

printf '{"inputs":{"run_mode":"full_escalation","expected_head":"%s"}}\n' "$head_oid" >"$tmp_dir/full-escalation-missing-base.json"
if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; GITHUB_EVENT_PATH="$tmp_dir/full-escalation-missing-base.json" GITHUB_EVENT_NAME=workflow_dispatch GITHUB_SHA="$head_oid" product_doc_range) >"$tmp_dir/full-escalation-missing-base.out" 2>&1; then
  echo "product-doc-content-callers.test: full escalation without integration base unexpectedly passed" >&2
  exit 1
fi
grep -Fq "did not provide both base/head OIDs" "$tmp_dir/full-escalation-missing-base.out"

if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; CI=true GITHUB_ACTIONS=true GITHUB_EVENT_PATH="" GITHUB_EVENT_NAME="" product_doc_range) >"$tmp_dir/no-event.out" 2>&1; then
  echo "product-doc-content-callers.test: CI without event/range unexpectedly fell back" >&2
  exit 1
fi
grep -Fq "CI requires explicit base/head OIDs" "$tmp_dir/no-event.out"

grep -Fq -- '--head "$SOURCE_HEAD" --worktree' ./scripts/prepare-task-pr.sh

sed -n '/^  full-regression:/,/^  full-escalation:/p' .github/workflows/rust.yml >"$tmp_dir/full-regression.yml"
sed -n '/^  full-escalation:/,$p' .github/workflows/rust.yml >"$tmp_dir/full-escalation.yml"
for full_workflow in "$tmp_dir/full-regression.yml" "$tmp_dir/full-escalation.yml"; do
  grep -Fq 'fetch-depth: 0' "$full_workflow"
  grep -Fq 'name: Resolve product-document gate range' "$full_workflow"
  grep -Fq "git rev-parse --verify 'HEAD^1'" "$full_workflow"
  grep -Fq "git rev-parse --verify 'HEAD^{commit}'" "$full_workflow"
  grep -Fq 'OASIS7_PRODUCT_DOC_BASE=' "$full_workflow"
  grep -Fq 'OASIS7_PRODUCT_DOC_HEAD=' "$full_workflow"
  grep -Fq 'GITHUB_ENV' "$full_workflow"
done
echo "product-doc-content-callers.test: OK"
