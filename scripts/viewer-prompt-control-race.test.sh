#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
prompt_runner="$repo_root/scripts/viewer-prompt-control-regression.sh"
race_runner="$repo_root/scripts/viewer-prompt-control-race.sh"

# PWT4-VIEWER-RED-010 is a contract-only RED gate.  It deliberately does not
# launch a browser or assert that two live actors can both qualify for one
# Agent; the hosted binding authority must define that boundary first.
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

# Minimal two-owned-session surface.  These are deliberately structural
# contracts only: no dual-actor authorization/eligibility assertion is made
# until the runtime auth boundary is provided by the owning specialist.
require_text "$race_runner" 'SESSION_A=' 'explicit actor A session'
require_text "$race_runner" 'SESSION_B=' 'explicit actor B session'
require_text "$race_runner" 'ARTIFACT_DIR_A=' 'separate actor A artifact directory'
require_text "$race_runner" 'ARTIFACT_DIR_B=' 'separate actor B artifact directory'
require_text "$race_runner" 'ab_session_cleanup "$SESSION_A"' 'actor A owned-session cleanup'
require_text "$race_runner" 'ab_session_cleanup "$SESSION_B"' 'actor B owned-session cleanup'
require_text "$race_runner" 'ab_read_retry' 'read-only retry helper'
require_text "$race_runner" 'write_safe_state' 'per-actor safe state capture'
require_text "$race_runner" 'request_id' 'request correlation field'
require_text "$race_runner" 'artifact-manifest.json' 'race artifact manifest'

if [[ -f "$race_runner" ]]; then
  if rg -n 'ab_read_retry[^\n]*(click|fill|submit|sendPromptControl)|ab_read_retry[^\n]*__AW_TEST__\.sendPromptControl' "$race_runner"; then
    failures+=$'\n- action-bearing prompt operations must not use the read retry helper'
  fi
  if rg -n 'sendPromptControl|__AW_TEST__\.sendPromptControl' "$race_runner"; then
    failures+=$'\n- prompt submission must remain a visible browser action, not a test API call'
  fi
fi

if [[ -n "$failures" ]]; then
  echo "viewer-prompt-control race/rollback contract: RED (expected)" >&2
  printf '%b\n' "$failures" >&2
  exit 1
fi

echo "viewer-prompt-control race/rollback contract: passed"
