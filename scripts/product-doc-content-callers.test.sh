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

if (unset OASIS7_PRODUCT_DOC_BASE OASIS7_PRODUCT_DOC_HEAD; CI=true GITHUB_ACTIONS=true GITHUB_EVENT_PATH="" GITHUB_EVENT_NAME="" product_doc_range) >"$tmp_dir/no-event.out" 2>&1; then
  echo "product-doc-content-callers.test: CI without event/range unexpectedly fell back" >&2
  exit 1
fi
grep -Fq "CI requires explicit base/head OIDs" "$tmp_dir/no-event.out"

grep -Fq -- '--head "$SOURCE_HEAD" --worktree' ./scripts/prepare-task-pr.sh
echo "product-doc-content-callers.test: OK"
