#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
race_runner="$repo_root/scripts/viewer-prompt-control-race.sh"
handoff_source="$repo_root/crates/oasis7_viewer/software_safe_src/viewer_browser_race_handoff_module.js"
viewer_core="$repo_root/crates/oasis7_viewer/software_safe_src/legacy_core.js"

# PWT-004 identity RED contract.  The authority decision is same player,
# session, and key across two real same-origin tabs.  This contract deliberately
# does not launch a browser and does not create or print any credential fixture.
failures=""
require_text() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if [[ ! -f "$file" ]]; then
    failures+=$'\n- '"$label: missing file $file"
  elif ! grep -Fq -- "$needle" "$file"; then
    failures+=$'\n- '"$label: missing $needle"
  fi
}

require_any_text() {
  local file="$1"
  local label="$2"
  shift 2
  if [[ ! -f "$file" ]]; then
    failures+=$'\n- '"$label: missing file $file"
    return
  fi
  local needle
  for needle in "$@"; do
    if grep -Fq -- "$needle" "$file"; then
      return
    fi
  done
  failures+=$'\n- '"$label: none of the required markers found"
}

# Runner semantics: one owned browser session, two stable tabs.  The
# old two-independent-session surface must not be reused for same-key proof.
require_text "$race_runner" 'SESSION=' 'shared owned browser session'
require_text "$race_runner" 'TAB_A=' 'actor A tab identity'
require_text "$race_runner" 'TAB_B=' 'actor B tab identity'
require_text "$race_runner" 'tab list --json' 'stable tab inventory'
require_text "$race_runner" 'tab new' 'same-origin second tab creation'
require_text "$race_runner" 'TAB_A' 'actor A tab selection'
require_text "$race_runner" 'TAB_B' 'actor B tab selection'

if [[ -f "$race_runner" ]] && grep -En 'SESSION_A=|SESSION_B=' "$race_runner"; then
  failures+=$'\n- same-key proof must not create independent SESSION_A/SESSION_B browser sessions'
fi

# The browser-instrumentation URL is test-only and loopback-only.  Hosted
# test-login and test_api are explicit requirements, not inferred defaults.
require_any_text "$race_runner" 'test API query opt-in' 'test_api=1' '"test_api": "1"'
require_any_text "$race_runner" 'hosted test-login query opt-in' 'hosted_test_login=1' '"hosted_test_login": "1"'
require_any_text "$race_runner" 'loopback URL validation' 'require_loopback_url' 'loopback-only' '127.0.0.1'

# Browser-internal handoff: unguessable run-scoped channel, same-origin check,
# expiry/replay protection, and close/dispose.  The handoff source is expected
# to stay test-only and in-memory; it must never persist or serialize secrets.
require_text "$handoff_source" 'BroadcastChannel' 'browser-internal handoff channel'
require_any_text "$handoff_source" 'cryptographic run nonce' 'crypto.getRandomValues' 'crypto.randomUUID' 'cryptoRef.getRandomValues' 'cryptoRef.randomUUID'
require_any_text "$handoff_source" 'run-scoped channel name' 'runNonce' 'handoffNonce' 'channelName'
require_any_text "$handoff_source" 'same-origin handoff validation' 'location.origin' 'event.origin' 'origin'
require_any_text "$handoff_source" 'one-time handoff expiry' 'expiresAt' 'ttl' 'expiration'
require_any_text "$handoff_source" 'handoff replay rejection' 'replay' 'consumed' 'used'
require_any_text "$handoff_source" 'handoff channel disposal' 'channel.close' 'dispose'
require_any_text "$handoff_source" 'test-only handoff gate' 'test_api' 'isTestApiEnabled'
require_any_text "$handoff_source" 'hosted test-login handoff gate' 'hosted_test_login' 'hostedTestLogin'
require_text "$viewer_core" 'offerBrowserRaceIdentityForTest' 'browser-only identity offer integration'
require_text "$viewer_core" 'claimBrowserRaceIdentityForTest' 'browser-only identity claim integration'
require_text "$viewer_core" 'connectBrowserRaceActorForTest' 'recipient connects only after identity claim'
require_text "$viewer_core" '1_000_000' 'disjoint actor B request and nonce range'

if [[ -f "$handoff_source" ]]; then
  if grep -En 'localStorage|sessionStorage|location\.(search|href)|console\.(log|warn|error)' "$handoff_source"; then
    failures+=$'\n- handoff implementation must not persist, put in URL, or log key material'
  fi
fi

# The runner must capture redacted state only; private key, release token, and
# registration grant are browser-only and must never enter artifacts.
require_text "$race_runner" 'sensitive_keys' 'safe-state redaction set'
require_text "$race_runner" 'privateKey' 'private-key redaction marker'
require_text "$race_runner" 'releaseToken' 'release-token redaction marker'
require_text "$race_runner" 'registrationGrant' 'registration-grant redaction marker'
require_text "$race_runner" 'write_safe_state' 'redacted actor-state capture'

if [[ -n "$failures" ]]; then
  echo "viewer-prompt-control race identity contract: RED (expected)" >&2
  printf '%b\n' "$failures" >&2
  exit 1
fi

echo "viewer-prompt-control race identity contract: passed"
