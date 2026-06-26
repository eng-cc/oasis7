#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

template="doc/testing/templates/state-sync-closure-evidence-packet-template.md"

require_literal() {
  local literal=$1
  if ! grep -Fq "$literal" "$template"; then
    echo "missing required template literal: $literal" >&2
    exit 1
  fi
}

require_regex() {
  local regex=$1
  if ! grep -Eq "$regex" "$template"; then
    echo "missing required template pattern: $regex" >&2
    exit 1
  fi
}

test -f "$template"

require_literal 'Supported claim | `module_full`'
require_literal 'Does not claim | `integration_required`, `release_full`, `public_testnet ready`, mainnet readiness'
require_literal 'Manual data/checkpoint/seed copy may be recorded as recovery context, but it cannot count as live state-sync closure.'
require_literal 'GWSC module_required'
require_literal 'Mixed topology full'
require_literal 'Triad soak'
require_literal 'State-sync closure'
require_literal 'observer_catch_up_completed'
require_literal 'missing_blob_count'
require_literal 'manual_copy_used'
require_literal 'Observer catch-up is recorded separately from blob closure.'
require_literal 'No manual data/checkpoint/seed copy is counted as live sync evidence.'

require_regex '^## Claim Boundary$'
require_regex '^### Command Evidence$'
require_regex '^### Commit Propagation$'
require_regex '^### Peer Heads And Gap Sync$'
require_regex '^### State-Sync Bundle And Blob Closure$'
require_regex '^### Failure Signatures$'
require_regex '^## Minimum Review Checklist$'

echo "state-sync closure evidence template smoke checks passed"
