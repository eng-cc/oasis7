#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
prompt_runner="$repo_root/scripts/viewer-prompt-control-regression.sh"
race_runner="$repo_root/scripts/viewer-prompt-control-race.sh"

# PWT4-RACE-L is a deterministic runner contract. It does not launch a
# browser/provider, but it fails closed unless the live lane records a pinned
# same-Chrome dispatch barrier and complete authority receipts.
failures=""
require_text() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if [[ ! -f "$file" ]]; then
    failures+=$'\n- '"$label: missing file $file"
  elif ! rg -Fq -- "$needle" "$file"; then
    failures+=$'\n- '"$label: missing $needle"
  fi
}

require_regex() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  if [[ ! -f "$file" ]]; then
    failures+=$'\n- '"$label: missing file $file"
  elif ! rg -q -- "$pattern" "$file"; then
    failures+=$'\n- '"$label: missing /$pattern/"
  fi
}

# Rollback must finish only after the authoritative response version is
# reflected by the selected Viewer state.  This must be a separate gate from
# the apply convergence gate and must precede rollback state/artifact capture.
require_text "$prompt_runner" \
  'wait_for_rollback_version_convergence' \
  'rollback authoritative convergence helper'
require_text "$prompt_runner" \
  'rollback authoritative version convergence' \
  'rollback convergence phase label'
require_text "$prompt_runner" \
  'Number(s?.selectedPromptVersion) === Number(r?.version)' \
  'rollback selected version equals authoritative response version'

if [[ -f "$prompt_runner" ]]; then
  if ! python3 - "$prompt_runner" <<'PY'
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(encoding="utf-8")
convergence = text.find("wait_for_rollback_version_convergence")
readback = text.find('write_safe_state "$rollback_state"')
if convergence < 0 or readback < 0 or convergence > readback:
    raise SystemExit(1)
PY
  then
    failures+=$'\n- rollback state capture must follow authoritative version convergence'
  fi
fi

# Minimal same-identity two-tab surface. The live lane must issue ordinary
# signed UI actions in both tabs; setup-only/readback scaffolding is not race
# evidence. These assertions are intentionally narrow so the implementation
# agent gets a deterministic RED handoff for visible Apply, equal baseline,
# and exactly-one applied/stale convergence.
require_text "$race_runner" 'SESSION=' 'single owned browser session'
require_text "$race_runner" 'TAB_A=' 'explicit actor A tab'
require_text "$race_runner" 'TAB_B=' 'explicit actor B tab'
require_text "$race_runner" 'ARTIFACT_DIR_A=' 'separate actor A artifact directory'
require_text "$race_runner" 'ARTIFACT_DIR_B=' 'separate actor B artifact directory'
require_text "$race_runner" 'ab_session_cleanup "$SESSION"' 'shared owned-session cleanup'
require_text "$race_runner" 'ab_read_retry' 'read-only retry helper'
require_text "$race_runner" 'write_safe_state' 'per-actor safe state capture'
require_text "$race_runner" 'request_id' 'request correlation field'
require_text "$race_runner" 'artifact-manifest.json' 'race artifact manifest'
require_text "$race_runner" 'umask 077' 'private evidence umask'
require_text "$race_runner" 'chmod 700' 'private evidence root'
require_text "$race_runner" 'cleanupFailure' 'cleanup failure evidence'
require_text "$race_runner" 'acceptanceInvalidated' 'cleanup acceptance invalidation'
require_text "$race_runner" 'write_manifest' 'post-cleanup manifest generation'
require_text "$race_runner" 'runtimeActionsDuringVerification' 'runtime action trace'
require_text "$race_runner" 'providerCallsDuringVerification": False' 'provider call prohibition'
require_text "$race_runner" 'authAuthorityEpoch' 'authority epoch baseline'
require_text "$race_runner" 'authPlayerId' 'player identity baseline'
require_text "$race_runner" 'authBoundAgentId' 'agent binding baseline'
require_text "$race_runner" 'authPublicKey' 'key identity baseline'
require_text "$race_runner" 'CDP_URL' 'pinned CDP dispatch'
require_text "$race_runner" '--pin-tab' 'strict tab pinning'
require_text "$race_runner" 'DISPATCH_RELEASE' 'shared dispatch barrier'
require_text "$race_runner" 'dispatchEligible' 'concurrency eligibility trace'
require_text "$race_runner" 'startedAtNs' 'action dispatch timing trace'
require_text "$race_runner" 'current_version' 'stale current version evidence'
require_text "$race_runner" 'applied_scope' 'runtime apply scope evidence'
require_text "$race_runner" 'persistence_scope' 'persistence scope evidence'
require_text "$race_runner" 'sync_scope' 'sync scope evidence'
require_text "$race_runner" 'mutation_count' 'mutation count evidence'
require_text "$race_runner" 'operation_digest' 'operation receipt evidence'

# Each actor must submit Apply through its pinned visible browser control.
if [[ -f "$race_runner" ]]; then
  apply_clicks=$(rg -F -c 'button[data-prompt-action="apply"]' "$race_runner" || true)
  if [[ "${apply_clicks:-0}" -lt 2 ]]; then
    failures+=$'\n- live race must issue two visible Apply clicks (one per tab)'
  fi
fi

# The runner must capture and assert one shared baseline/version before the
# clicks, then correlate both signed requests to that same expected version.
require_text "$race_runner" 'baseline_version' 'captured common baseline version'
require_text "$race_runner" 'same_baseline' 'same-baseline assertion'
require_text "$race_runner" 'expected_version' 'per-request expected version correlation'

# The acceptance artifact must prove exactly one authority winner and one stale
# loser. Counts belong in the runner contract, not in prose-only evidence.
require_text "$race_runner" 'outcomeCounts' 'race outcome counts'
require_text "$race_runner" '"applied": 1' 'exactly one applied outcome'
require_text "$race_runner" '"stale": 1' 'exactly one stale outcome'
require_text "$race_runner" '"promptSubmission": "apply"' 'visible apply receipt'
require_text "$race_runner" 'mutations[stale_index] in (None, 0)' 'stale has no mutation'
require_text "$race_runner" 'mutations[applied_index] == 1' 'applied has one mutation'
require_text "$race_runner" 'statuses == outcomes' 'wire status matches browser outcome'
require_text "$race_runner" 'response_versions == [baseline_version + 1, baseline_version + 1]' 'authoritative version evidence'
require_text "$race_runner" 'request_ids == response_request_ids' 'request response correlation'
require_text "$race_runner" 'response_authorities' 'response authority evidence'
require_text "$race_runner" 'response_sessions' 'response session evidence'
require_text "$race_runner" 'response_bindings' 'response binding evidence'

if [[ -f "$race_runner" ]]; then
  if rg -n 'ab_read_retry[^\n]*(click|fill|submit|sendPromptControl)|ab_read_retry[^\n]*__AW_TEST__\.sendPromptControl' "$race_runner"; then
    failures+=$'\n- action-bearing prompt operations must not use the read retry helper'
  fi
  if rg -n 'sendPromptControl|__AW_TEST__\.sendPromptControl' "$race_runner"; then
    failures+=$'\n- prompt submission must remain a visible browser action, not a test API call'
  fi
fi

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/viewer-prompt-control-race-contract.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT
contract_out="$tmp_root/out"
if ! "$race_runner" --contract-only --out-dir "$contract_out" >"$tmp_root/contract.log" 2>&1; then
  failures+=$'\n- contract-only runner must remain executable after hardening'
else
  python3 - "$contract_out" <<'PY'
import json
import os
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
manifest = json.loads((root / "artifact-manifest.json").read_text(encoding="utf-8"))
assert manifest["acceptanceEligible"] is False
assert manifest["providerCallsDuringVerification"] is False
assert manifest["runtimeActionsDuringVerification"] == []
assert manifest["cleanup"]["status"] == "passed"
assert manifest["cleanup"]["acceptanceInvalidated"] is False
assert (root / "cleanup.json").is_file()
assert (root / "cleanup.json").name in {entry["path"] for entry in manifest["artifacts"]}
assert os.stat(root).st_mode & 0o777 == 0o700
for actor in (root / "actor-a", root / "actor-b"):
    assert os.stat(actor).st_mode & 0o777 == 0o700
PY
fi

if [[ -n "$failures" ]]; then
  echo "viewer-prompt-control race/rollback contract: RED (expected)" >&2
  printf '%b\n' "$failures" >&2
  exit 1
fi

echo "viewer-prompt-control race/rollback contract: passed"
