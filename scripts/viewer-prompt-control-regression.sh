#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASE_ID="PWT-004"
OUT_DIR="output/playwright/prompt-control/${CASE_ID}"
HEADED=0
CONTRACT_ONLY=0
FULL_GAMEPLAY=0
GAME_URL=""
AGENT_ID="starter-agent-0"
PROMPT_GOAL="Inspect the selected agent's prompt control state."
STARTUP_TIMEOUT=180
ACTION_TIMEOUT_MS=10000
TEST_LOGIN=0
STACK_ARGS=()
STACK_BOOTSTRAPPED=0
STACK_CHAIN_ARG_EXPLICIT=0
FIRST_AGENT_CLAIM_PERFORMED=0

usage() {
  cat <<'EOF'
Usage: viewer-prompt-control-regression.sh --headed [options]

Runs the PWT-004 prompt-control flow in a headed browser.  --contract-only
produces the deterministic runner contract and manifest without launching a
browser, game process, or model provider.

Options:
  --headed | --headless
  --contract-only
  --full-gameplay         launch the trusted-local chain-backed gameplay lane
  --case-id ID
  --out-dir DIR
  --url URL
  --test-login             use the visible local hosted test-login flow
  --agent-id ID
  --prompt-goal TEXT
  --startup-timeout SECONDS
  --action-timeout-ms MILLISECONDS
EOF
}

while (($# > 0)); do
  case "$1" in
    --headed) HEADED=1 ;;
    --headless) HEADED=0 ;;
    --contract-only) CONTRACT_ONLY=1 ;;
    --full-gameplay) FULL_GAMEPLAY=1 ;;
    --case-id) shift; CASE_ID="${1:?missing value for --case-id}" ;;
    --out-dir) shift; OUT_DIR="${1:?missing value for --out-dir}" ;;
    --url) shift; GAME_URL="${1:?missing value for --url}" ;;
    --test-login) TEST_LOGIN=1 ;;
    --agent-id) shift; AGENT_ID="${1:?missing value for --agent-id}" ;;
    --prompt-goal) shift; PROMPT_GOAL="${1:?missing value for --prompt-goal}" ;;
    --startup-timeout) shift; STARTUP_TIMEOUT="${1:?missing value for --startup-timeout}" ;;
    --action-timeout-ms) shift; ACTION_TIMEOUT_MS="${1:?missing value for --action-timeout-ms}" ;;
    -h|--help) usage; exit 0 ;;
    *) STACK_ARGS+=("$1") ;;
  esac
  shift
done

if ((${#STACK_ARGS[@]} > 0)); then
  for stack_arg in "${STACK_ARGS[@]}"; do
    # An explicit chain argument from the caller is authoritative.
    if [[ "$stack_arg" == --chain-* ]]; then
      STACK_CHAIN_ARG_EXPLICIT=1
      break
    fi
  done
fi

if (( HEADED == 0 )); then
  echo "error: --headed is required for PWT-004; headed evidence is mandatory" >&2
  exit 2
fi
if [[ "$CASE_ID" != "PWT-004" ]]; then
  echo "error: this runner currently supports only --case-id PWT-004" >&2
  exit 2
fi
if [[ ! "$AGENT_ID" =~ ^[A-Za-z0-9_.:-]+$ ]]; then
  echo "error: --agent-id contains unsupported characters" >&2
  exit 2
fi
if ! [[ "$STARTUP_TIMEOUT" =~ ^[1-9][0-9]*$ && "$ACTION_TIMEOUT_MS" =~ ^[1-9][0-9]*$ ]]; then
  echo "error: timeouts must be positive integers" >&2
  exit 2
fi

LOCAL_PROVIDER_AUTHORITY="$OUT_DIR/runtime/local-test-provider-authority.json"
LOCAL_PROVIDER_WASM="$ROOT_DIR/.tmp/wasm-build-suite/module.runtime.local-test-provider.wasm"
LOCAL_PROVIDER_METADATA="$ROOT_DIR/.tmp/wasm-build-suite/module.runtime.local-test-provider.metadata.json"

if (( FULL_GAMEPLAY == 1 )); then
  if [[ ! -f "$LOCAL_PROVIDER_WASM" ]]; then
    echo "error: local test provider artifact WASM is missing: $LOCAL_PROVIDER_WASM" >&2
    exit 2
  fi
  if [[ ! -f "$LOCAL_PROVIDER_METADATA" ]]; then
    echo "error: local test provider artifact metadata is missing: $LOCAL_PROVIDER_METADATA" >&2
    exit 2
  fi
fi

AGENT_SELECTOR="[data-pixel-world-agent-marker=\"true\"][data-agent-id=\"${AGENT_ID}\"]"

require_nonempty_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "error: required strong-auth environment variable ${name} is missing" >&2
    exit 2
  fi
}

require_loopback_url() {
  local url="$1"
  if ! python3 - "$url" <<'PY'
import ipaddress
import sys
from urllib.parse import urlsplit

parts = urlsplit(sys.argv[1])
host = (parts.hostname or "").strip().lower()
if host == "localhost":
    raise SystemExit(0)
try:
    is_loopback = ipaddress.ip_address(host).is_loopback
except ValueError:
    is_loopback = False
raise SystemExit(0 if is_loopback else 1)
PY
  then
    echo "error: --test-login is restricted to a loopback URL" >&2
    exit 2
  fi
}

read_authoritative_hosted_url() {
  local runtime_log="$OUT_DIR/runtime/oasis7_viewer_live.log"
  python3 - "$runtime_log" <<'PY'
import pathlib
import re
import sys
from urllib.parse import parse_qsl, urlsplit

path = pathlib.Path(sys.argv[1])
if not path.is_file():
    raise SystemExit(0)
for line in path.read_text(errors="replace").splitlines():
    match = re.search(r"^- URL: (https?://\S+)", line)
    if not match:
        continue
    url = match.group(1)
    query = dict(parse_qsl(urlsplit(url).query, keep_blank_values=True))
    if query.get("hosted_access", "").strip():
        print(url)
        raise SystemExit(0)
raise SystemExit(0)
PY
}

wait_for_authoritative_hosted_url() {
  local deadline=$((SECONDS + STARTUP_TIMEOUT))
  local authoritative_url
  while (( SECONDS < deadline )); do
    authoritative_url="$(read_authoritative_hosted_url)"
    if [[ -n "$authoritative_url" ]]; then
      printf '%s\n' "$authoritative_url"
      return 0
    fi
    if [[ -n "$LAUNCH_PID" ]] && ! kill -0 "$LAUNCH_PID" 2>/dev/null; then
      echo "error: hosted_public_join launcher exited before publishing authoritative hosted URL" >&2
      return 1
    fi
    sleep 1
  done
  echo "error: authoritative hosted URL with hosted_access was not published under $OUT_DIR/runtime/oasis7_viewer_live.log" >&2
  return 1
}

meta_value() {
  local key="$1"
  local path="$2"
  sed -n "s/^${key}=//p" "$path" | tail -n 1
}

wait_for_full_gameplay_readiness() {
  local stack_meta="$OUT_DIR/runtime/session.meta"
  local deadline=$((SECONDS + STARTUP_TIMEOUT))
  while (( SECONDS < deadline )); do
    if [[ -n "$LAUNCH_PID" ]] && ! kill -0 "$LAUNCH_PID" 2>/dev/null; then
      echo "error: full-gameplay launcher exited before readiness" >&2
      return 1
    fi
    if [[ -f "$stack_meta" ]] \
      && [[ "$(meta_value STACK_READY "$stack_meta")" == "1" ]] \
      && [[ "$(meta_value LOCAL_TEST_PROVIDER_SETUP_ENABLED "$stack_meta")" == "1" ]] \
      && [[ "$(meta_value CHAIN_ENABLED "$stack_meta")" == "1" ]] \
      && [[ "$(meta_value DEPLOYMENT_MODE "$stack_meta")" == "trusted_local_only" ]] \
      && [[ -f "$LOCAL_PROVIDER_AUTHORITY" ]]; then
      if python3 - "$LOCAL_PROVIDER_AUTHORITY" "$AGENT_ID" <<'PY'
import json
import pathlib
import sys

authority_path = pathlib.Path(sys.argv[1])
agent_id = sys.argv[2]
try:
    authority = json.loads(authority_path.read_text(encoding="utf-8"))
except (OSError, ValueError):
    raise SystemExit(1)
authority_grant = authority.get("grant")
capability_invocation_context = authority.get("invocation_context")
if not isinstance(authority_grant, dict) or not isinstance(capability_invocation_context, dict):
    raise SystemExit(1)
if authority.get("agent_id") != agent_id:
    raise SystemExit(1)
if not authority_grant.get("grant_id") or authority_grant.get("grant_id") != capability_invocation_context.get("grant_id"):
    raise SystemExit(1)
subject = capability_invocation_context.get("subject")
if isinstance(subject, dict) and subject.get("agent_id") not in (None, agent_id):
    raise SystemExit(1)
PY
      then
        return 0
      fi
    fi
    sleep 1
  done
  echo "error: full-gameplay readiness requires STACK_READY=1, LOCAL_TEST_PROVIDER_SETUP_ENABLED=1, CHAIN_ENABLED=1, DEPLOYMENT_MODE=trusted_local_only, and consistent authority_grant/capability_invocation_context" >&2
  return 1
}

configure_loopback_browser_args() {
  local args
  if [[ ${AGENT_BROWSER_ARGS+x} ]]; then
    args="$AGENT_BROWSER_ARGS"
  else
    args="$(ab_browser_args)"
    if [[ "$(uname -s)" == "Darwin" ]]; then
      # The library default is GL, but headed macOS Chrome exposes no WebGL2
      # canvas with that backend. Preserve an explicit caller override.
      args="${args//--use-angle=gl/--use-angle=metal}"
    fi
  fi
  case ",$args," in
    *,--no-proxy-server,*) ;;
    *)
      [[ -n "$args" ]] && args+=","
      args+="--no-proxy-server"
      ;;
  esac
  AGENT_BROWSER_ARGS="$args"
  export AGENT_BROWSER_ARGS
}

visible_action_contract() {
  cat <<'EOF'
{"selection":"[data-pixel-world-agent-marker=\"true\"][data-agent-id=\"${AGENT_ID}\"]","promptDisclosure":"Advanced Prompt Settings","shortTermGoal":"#prompt-short","testLogin":"[data-auth-action=\"test-login\"]","approvalCode":"#strong-auth-approval-code","preview":"button[data-prompt-action=\"preview\"]","apply":"button[data-prompt-action=\"apply\"]","rollbackTarget":"#prompt-rollback-version","rollback":"button[data-prompt-action=\"rollback\"]"}
EOF
}

write_manifest() {
  local root="$1"
  local tier="$2"
  local eligible="$3"
  local provider_calls="$4"
  local agent_id="${5:-\${AGENT_ID}}"
  python3 - "$root" "$CASE_ID" "$tier" "$eligible" "$provider_calls" "$agent_id" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
case_id, tier = sys.argv[2], sys.argv[3]
eligible = sys.argv[4].lower() == "true"
provider_calls = sys.argv[5].lower() == "true"
agent_id = sys.argv[6]
visible = {
    "selection": rf'[data-pixel-world-agent-marker=\"true\"][data-agent-id=\"{agent_id}\"]',
    "promptDisclosure": "Advanced Prompt Settings",
    "shortTermGoal": "#prompt-short",
    "testLogin": r'[data-auth-action=\"test-login\"]',
    "approvalCode": "#strong-auth-approval-code",
    "preview": r'button[data-prompt-action=\"preview\"]',
    "apply": r'button[data-prompt-action=\"apply\"]',
    "rollbackTarget": "#prompt-rollback-version",
    "rollback": r'button[data-prompt-action=\"rollback\"]',
}
artifacts = []
for path in sorted(root.rglob("*")):
    if not path.is_file() or path.name == "artifact-manifest.json":
        continue
    data = path.read_bytes()
    artifacts.append({
        "bytes": len(data),
        "path": path.relative_to(root).as_posix(),
        "sha256": hashlib.sha256(data).hexdigest(),
    })
manifest = {
    "acceptanceEligible": eligible,
    "artifacts": artifacts,
    "browserMode": "headed",
    "caseId": case_id,
    "evidenceTier": tier,
    "providerCallsDuringVerification": provider_calls,
    "schema": "oasis7.viewer.prompt-control-artifact-manifest/v1",
    "visibleActionContract": visible,
}
(root / "artifact-manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY
}

write_contract_artifacts() {
  mkdir -p "$OUT_DIR"
  python3 - "$OUT_DIR" "$CASE_ID" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
case_id = sys.argv[2]
visible = {
    "selection": r'[data-pixel-world-agent-marker=\"true\"][data-agent-id=\"${AGENT_ID}\"]',
    "promptDisclosure": "Advanced Prompt Settings",
    "shortTermGoal": "#prompt-short",
    "testLogin": "[data-auth-action=\"test-login\"]",
    "approvalCode": "#strong-auth-approval-code",
    "preview": r'button[data-prompt-action=\"preview\"]',
    "apply": r'button[data-prompt-action=\"apply\"]',
    "rollbackTarget": "#prompt-rollback-version",
    "rollback": r'button[data-prompt-action=\"rollback\"]',
}
contract = {
    "browserMode": "headed",
    "caseId": case_id,
    "providerCallsDuringVerification": False,
    "visibleActionContract": visible,
}
manifest_input = {
    "acceptanceEligible": False,
    "evidenceTier": "contract_only",
    "required": ["headed_browser", "visible_selection", "preview", "apply", "rollback"],
}
(root / "contract.json").write_text(json.dumps(contract, indent=2, sort_keys=True) + "\n", encoding="utf-8")
(root / "manifest-input.json").write_text(json.dumps(manifest_input, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
  write_manifest "$OUT_DIR" "contract_only" false false
  echo "contract-only artifacts written to $OUT_DIR"
}

# This branch is deliberately before ab_require and the launcher. It is the
# deterministic, no-provider verification surface used by automated tests.
if (( CONTRACT_ONLY == 1 )); then
  write_contract_artifacts
  exit 0
fi

require_nonempty_env OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY
require_nonempty_env OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY
require_nonempty_env OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE
if (( TEST_LOGIN == 0 )); then
  echo "error: PWT-004 requires explicit --test-login for the local strong-auth evidence lane" >&2
  exit 2
fi
export OASIS7_HOSTED_TEST_LOGIN_ENABLED=1
if [[ -n "$GAME_URL" ]]; then
  require_loopback_url "$GAME_URL"
fi

source "$ROOT_DIR/scripts/agent-browser-lib.sh"
mkdir -p "$OUT_DIR"
RUN_ID="viewer-prompt-control-${CASE_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
LAUNCH_LOG="$OUT_DIR/launcher.log"
AB_LOG="$OUT_DIR/agent-browser.log"
LAUNCH_PID=""
SESSION=""
cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$LAUNCH_PID" ]] && kill -0 "$LAUNCH_PID" 2>/dev/null; then
    kill "$LAUNCH_PID" 2>/dev/null || true
    wait "$LAUNCH_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$SESSION" ]]; then
    ab_session_cleanup "$SESSION" "$OUT_DIR" || ab_cmd "$SESSION" close >/dev/null 2>&1 || true
  fi
  exit "$exit_code"
}
trap cleanup EXIT

ab_require
SESSION="$(ab_session_begin "viewer-prompt-control-${CASE_ID}-${RUN_ID}" "$OUT_DIR")"

diagnostic_slug() {
  printf '%s' "$1" | tr -cs '[:alnum:]_.-' '_' | sed 's/^_//; s/_$//'
}

capture_failure_diagnostics() {
  local stage="$1"
  local slug
  local base
  local page_summary
  local state
  slug="$(diagnostic_slug "$stage")"
  base="$OUT_DIR/failure-${slug}"

  printf '[failure:%s] collecting browser diagnostics\n' "$stage" >>"$AB_LOG"
  AB_READ_RETRY_ATTEMPTS=1 ab_read_retry "$SESSION" session info --json \
    >"${base}-session-info.json" 2>&1 || true
  AB_READ_RETRY_ATTEMPTS=1 ab_read_retry "$SESSION" tab list --json \
    >"${base}-tabs.json" 2>&1 || true
  AB_READ_RETRY_ATTEMPTS=1 ab_read_retry "$SESSION" console \
    >"${base}-console.log" 2>&1 || true
  AB_READ_RETRY_ATTEMPTS=1 ab_read_retry "$SESSION" errors \
    >"${base}-errors.log" 2>&1 || true
  AB_READ_RETRY_ATTEMPTS=1 ab_read_retry "$SESSION" snapshot -i \
    >"${base}-snapshot.txt" 2>&1 || true
  ab_screenshot "$SESSION" "${base}.png" \
    >"${base}-screenshot.log" 2>&1 || true

  page_summary="$(AB_READ_RETRY_ATTEMPTS=1 ab_read_eval "$SESSION" \
    'JSON.stringify({readyState:document.readyState,title:document.title,url:location.href,awTest:typeof window.__AW_TEST__,testLogin:Boolean(document.querySelector("[data-auth-action=\\"test-login\\"]"))})' \
    2>/dev/null || true)"
  printf '%s\n' "$page_summary" >"${base}-page-summary.json"

  state="$(AB_READ_RETRY_ATTEMPTS=1 ab_read_eval "$SESSION" \
    'window.__AW_TEST__?.getState?.() ?? null' 2>/dev/null || true)"
  if [[ -n "$state" ]]; then
    write_safe_state "$state" "${base}-state.json" || true
  fi
  printf '[failure:%s] diagnostics written under %s\n' "$stage" "$OUT_DIR" >>"$AB_LOG"
}

wait_for_cli_stage() {
  local stage="$1"
  shift
  local defer_failure=0
  local output
  local result
  if [[ "${1:-}" == "--defer-failure" ]]; then
    defer_failure=1
    shift
  fi
  if output="$(ab_read_retry "$SESSION" "$@" 2>&1)"; then
    printf '[%s] %s\n' "$stage" "$output" >>"$AB_LOG"
    return 0
  else
    result=$?
  fi
  printf '[%s] command failed (exit=%s):\n%s\n' "$stage" "$result" "$output" >>"$AB_LOG"
  capture_failure_diagnostics "$stage"
  if (( defer_failure == 0 )); then
    echo "error: ${stage} wait failed (phase: ${stage}; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "$stage")-*)" >&2
  fi
  return "$result"
}

run_visible_action() {
  local phase="$1"
  shift
  local output
  local result
  # Visible actions intentionally call ab_cmd directly: an uncertain click or
  # fill must never be replayed because it may already have reached the page.
  if output="$(ab_cmd "$SESSION" "$@" 2>&1)"; then
    printf '[action:%s] %s\n' "$phase" "$output" >>"$AB_LOG"
    return 0
  else
    result=$?
  fi
  printf '[action:%s] command failed (exit=%s):\n%s\n' "$phase" "$result" "$output" >>"$AB_LOG"
  capture_failure_diagnostics "$phase"
  echo "error: visible action ${phase} failed (phase: ${phase}; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "$phase")-*)" >&2
  return "$result"
}

register_hosted_player_session() {
  local result
  # This hook only registers the server-side session; it is intentionally an
  # action-bearing eval with no retry because it may already have committed.
  if ab_eval "$SESSION" "window.__AW_TEST__.registerPlayerSessionForTest(null)" >>"$AB_LOG" 2>&1; then
    printf '[action:hosted player session registration action] completed\n' >>"$AB_LOG"
    return 0
  else
    result=$?
  fi
  printf '[action:hosted player session registration action] command failed (exit=%s)\n' "$result" >>"$AB_LOG"
  capture_failure_diagnostics "hosted player session registration action"
  echo "error: hosted player session registration action failed (phase: hosted player session registration action; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "hosted player session registration action")-*)" >&2
  return "$result"
}

maybe_rebind_post_onboarding_session() {
  local agent_id_json="$1"
  local rebind_required
  local result
  rebind_required="$(ab_read_eval "$SESSION" "(() => { const s = window.__AW_TEST__.getState(); return s?.authBoundAgentId === ${agent_id_json} && s?.authBindingEpoch == null; })()" 2>/dev/null || true)"
  case "$rebind_required" in
    true|\"true\")
      # This is an action-bearing session setup call.  Never retry it: the
      # first request may have committed even if browser transport failed.
      if ab_eval "$SESSION" "window.__AW_TEST__.registerPlayerSessionForTest(${agent_id_json}, {forceRebind: true})" >>"$AB_LOG" 2>&1; then
        printf '[action:post-onboarding auth binding rebind action] completed\n' >>"$AB_LOG"
        return 0
      else
        result=$?
      fi
      printf '[action:post-onboarding auth binding rebind action] command failed (exit=%s)\n' "$result" >>"$AB_LOG"
      capture_failure_diagnostics "post-onboarding auth binding rebind action"
      echo "error: post-onboarding auth binding rebind action failed (phase: post-onboarding auth binding rebind action; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "post-onboarding auth binding rebind action")-*)" >&2
      return "$result"
      ;;
    false|\"false\")
      printf '[action:post-onboarding auth binding rebind action] skipped; binding epoch already present or agent not bound\n' >>"$AB_LOG"
      ;;
    *)
      capture_failure_diagnostics "post-onboarding auth binding rebind decision"
      echo "error: post-onboarding auth binding rebind state was unavailable (phase: post-onboarding auth binding rebind decision; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "post-onboarding auth binding rebind decision")-*)" >&2
      return 1
      ;;
  esac
}

maybe_claim_first_agent() {
  local agent_selector_json="$1"
  local empty_world
  local claim_button_xpath
  local claim_button_xpath_json
  empty_world="$(ab_read_eval "$SESSION" "(() => { const s = window.__AW_TEST__.getState(); const g = s?.gameplaySummary || {}; return !document.querySelector(${agent_selector_json}) && Number(g?.entityCounts?.agents) === 0; })()" 2>/dev/null || true)"
  case "$empty_world" in
    true|\"true\")
      FIRST_AGENT_CLAIM_PERFORMED=1
      ;;
    false|\"false\")
      return 0
      ;;
    *)
      capture_failure_diagnostics "empty-world entity gate"
      echo "error: empty-world entity gate state was unavailable (phase: empty-world entity gate)" >&2
      return 1
      ;;
  esac

  run_visible_action "claim first agent panel navigation action" click \
    'a[href="#viewer-targets-panel"]'
  claim_button_xpath='//*[@id="viewer-targets-panel"]//button[contains(normalize-space(.), "Claim First Agent") or contains(normalize-space(.), "认领第一个 Agent")]'
  claim_button_xpath_json="$(json_quote "$claim_button_xpath")"
  wait_for_cli_stage "claim first agent button" wait --fn \
    "Boolean((() => { const node = document.evaluate(${claim_button_xpath_json}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue; return node && !node.disabled; })())"
  run_visible_action "claim first agent action" click "xpath=$claim_button_xpath"
  wait_for_js_true \
    '(() => { const s = window.__AW_TEST__.getState(); const f = s?.lastGameplayActionFeedback; return f?.kind === "gameplay_action" && f?.action === "claim_first_agent" && f?.stage === "ack" && f?.accepted === true && f?.response?.action_id === "claim_first_agent"; })()' \
    "claim first agent gameplay authority ack"
  wait_for_js_true \
    '(() => { const s = window.__AW_TEST__.getState(); return Number(s?.gameplaySummary?.entityCounts?.agents) > 0; })()' \
    "claim first agent snapshot entity count"
  wait_for_js_true "(() => window.__AW_TEST__.getState()?.selectedId === ${agent_selector_json})()" \
    "claim first agent authoritative selection"
}

maybe_claim_starter_oc() {
  local agent_id_json="$1"
  local starter_oc_xpath
  local starter_oc_xpath_json
  local onboarding_visible
  # The player-facing overlay is titled Claim Your First OC.
  starter_oc_xpath='//*[@data-viewer-fixture-state="starter_oc_required_gate"]//button[contains(normalize-space(.), "Claim Starter OC") or contains(normalize-space(.), "领取初始 OC")]'
  starter_oc_xpath_json="$(json_quote "$starter_oc_xpath")"
  onboarding_visible="$(ab_read_eval "$SESSION" "Boolean((() => { const node = document.evaluate(${starter_oc_xpath_json}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue; return node && !node.disabled && node.getClientRects().length > 0; })())" 2>/dev/null || true)"
  case "$onboarding_visible" in
    false|\"false\")
      return 0
      ;;
    true|\"true\")
      ;;
    *)
      capture_failure_diagnostics "starter OC onboarding visibility"
      echo "error: starter OC onboarding visibility was unavailable (phase: starter OC onboarding visibility)" >&2
      return 1
      ;;
  esac

  run_visible_action "claim starter oc action" click "xpath=$starter_oc_xpath"
  wait_for_js_true \
    '(() => { const s = window.__AW_TEST__.getState(); const f = s?.lastGameplayActionFeedback; return f?.kind === "gameplay_action" && f?.action === "claim_starter_oc" && f?.stage === "ack" && f?.accepted === true && f?.response?.action_id === "claim_starter_oc"; })()' \
    "claim starter oc gameplay authority ack"
  wait_for_js_true "(() => { const s = window.__AW_TEST__.getState(); return !document.querySelector('[data-viewer-fixture-state=\"starter_oc_required_gate\"]') && Boolean(document.querySelector('#prompt-short')) && s?.authBoundAgentId === ${agent_id_json}; })()" \
    "starter OC overlay dismissal and agent binding"
}

wait_for_js_true() {
  local script="$1"
  local label="$2"
  local timeout_ms="${3:-$ACTION_TIMEOUT_MS}"
  local timeout_secs=$(( (timeout_ms + 999) / 1000 ))
  local deadline
  local value
  (( timeout_secs > 0 )) || timeout_secs=1
  deadline=$((SECONDS + timeout_secs))
  while (( SECONDS < deadline )); do
    value="$(ab_read_eval "$SESSION" "$script" 2>/dev/null || true)"
    if [[ "$value" == "true" || "$value" == '"true"' ]]; then
      return 0
    fi
    sleep 0.2
  done
  printf '[%s] JS wait timed out; last value=%s\n' "$label" "${value:-<empty>}" >>"$AB_LOG"
  capture_failure_diagnostics "$label"
  echo "error: timed out waiting for ${label} (phase: ${label}; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "$label")-*)" >&2
  return 1
}

wait_for_prompt_surface_continuity() {
  local timeout_ms="${1:-$ACTION_TIMEOUT_MS}"
  local timeout_secs=$(( (timeout_ms + 999) / 1000 ))
  local deadline
  local status=""
  (( timeout_secs > 0 )) || timeout_secs=1
  deadline=$((SECONDS + timeout_secs))
  while (( SECONDS < deadline )); do
    status="$(ab_read_eval "$SESSION" '(() => { const href = String(window.location.href || ""); if (href === "about:blank") return "tab_lost"; const panel = document.querySelector(`section#viewer-details-panel[data-viewer-route-panel="command"]`); const toggle = panel?.querySelector(`[data-prompt-visibility-toggle="1"]`); return panel && toggle ? "ready" : `prompt_surface_missing:${href}`; })()' 2>/dev/null || true)"
    case "$status" in
      ready|\"ready\")
        printf '[prompt surface page continuity] command panel and prompt visibility toggle ready\n' >>"$AB_LOG"
        return 0
        ;;
      tab_lost|\"tab_lost\")
        printf '[prompt surface page continuity] tab lost to about:blank\n' >>"$AB_LOG"
        capture_failure_diagnostics "pre-prompt tab loss"
        echo "error: pre-prompt tab loss detected: active page is about:blank (phase: pre-prompt tab loss; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "pre-prompt tab loss")-*)" >&2
        return 1
        ;;
    esac
    sleep 0.2
  done
  printf '[prompt surface page continuity] exact prompt DOM missing; last status=%s\n' "${status:-<empty>}" >>"$AB_LOG"
  capture_failure_diagnostics "prompt surface page continuity"
  echo "error: prompt surface missing while page remained active (phase: prompt surface page continuity; last status: ${status:-<empty>}; diagnostics: ${OUT_DIR}/failure-$(diagnostic_slug "prompt surface page continuity")-*)" >&2
  return 1
}

wait_for_domcontentloaded() {
  local result
  if wait_for_cli_stage "domcontentloaded" --defer-failure wait --load domcontentloaded; then
    return 0
  else
    result=$?
  fi
  if wait_for_js_true \
    'document.readyState === "complete" || document.readyState === "interactive"' \
    "domcontentloaded fallback" "$ACTION_TIMEOUT_MS"; then
    printf '[domcontentloaded fallback] JS readyState accepted after CLI wait failure\n' >>"$AB_LOG"
    echo "warning: domcontentloaded CLI wait failed; readyState fallback passed (phase: domcontentloaded fallback)" >&2
    return 0
  fi
  echo "error: domcontentloaded readiness failed after CLI wait and JS fallback (phase: domcontentloaded)" >&2
  return "$result"
}

wait_for_webgl2() {
  wait_for_js_true \
    'Boolean(document.createElement("canvas").getContext("webgl2"))' \
    "WebGL2 readiness" "$ACTION_TIMEOUT_MS"
}

state_raw() {
  ab_read_eval "$SESSION" 'window.__AW_TEST__.getState()'
}

write_safe_state() {
  local raw_json="$1"
  local out_path="$2"
  python3 - "$raw_json" "$out_path" <<'PY'
import json
import pathlib
import sys

raw, out_path = sys.argv[1], pathlib.Path(sys.argv[2])
sensitive_keys = {
    "authPlayerId",
    "authPublicKey",
    "authRevokedBy",
    "authRecoveryErrorMessage",
    "privateKey",
    "publicKey",
    "releaseToken",
    "registrationGrant",
    "approvalCode",
    "submittedDraft",
    "systemPrompt",
    "shortTermGoal",
    "longTermGoal",
    "system_prompt_override",
    "short_term_goal_override",
    "long_term_goal_override",
}

def redact(value):
    if isinstance(value, dict):
        return {
            key: redact(item)
            for key, item in value.items()
            if key not in sensitive_keys
        }
    if isinstance(value, list):
        return [redact(item) for item in value]
    return value

try:
    data = json.loads(raw)
except Exception:
    data = {"stateReadback": "unavailable"}
out_path.write_text(json.dumps(redact(data), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}

wait_for_prompt_feedback() {
  local mode="$1"
  local rollback_version="${2:-}"
  local expression
  local agent_id_json
  agent_id_json="$(json_quote "$AGENT_ID")"
  case "$mode" in
    preview)
      expression="(() => { const s = window.__AW_TEST__.getState(); const f = s?.lastPromptFeedback; const r = f?.response || {}; return f?.action === 'prompt_preview' && f?.stage === 'accepted' && r.status === 'accepted' && r.preview === true && Number(r.mutation_count) === 0 && r.applied_scope === 'none' && r.persistence_scope === 'none' && r.sync_scope === 'none' && s?.strongAuthLastGrantActionId === 'prompt_control_preview' && !s?.strongAuthLastGrantError && f?.agentId === ${agent_id_json}; })()"
      ;;
    apply)
      expression="(() => { const s = window.__AW_TEST__.getState(); const f = s?.lastPromptFeedback; const r = f?.response || {}; return f?.action === 'prompt_apply' && f?.stage === 'applied' && r.status === 'applied' && r.preview === false && Number(r.mutation_count) === 1 && r.applied_scope === 'runtime_instance' && r.persistence_scope === 'none' && r.sync_scope === 'none' && s?.strongAuthLastGrantActionId === 'prompt_control_apply' && !s?.strongAuthLastGrantError && f?.agentId === ${agent_id_json}; })()"
      ;;
    rollback)
      expression="(() => { const s = window.__AW_TEST__.getState(); const f = s?.lastPromptFeedback; const r = f?.response || {}; return f?.action === 'prompt_rollback' && f?.stage === 'applied' && r.status === 'applied' && r.operation === 'rollback' && Number(r.mutation_count) === 1 && r.applied_scope === 'runtime_instance' && r.persistence_scope === 'none' && r.sync_scope === 'none' && Number(r.rolled_back_to_version) === ${rollback_version} && s?.strongAuthLastGrantActionId === 'prompt_control_rollback' && !s?.strongAuthLastGrantError && f?.agentId === ${agent_id_json}; })()"
      ;;
    *)
      echo "error: unsupported prompt feedback mode ${mode}" >&2
      return 2
      ;;
  esac
  wait_for_js_true "$expression" "${mode} authority feedback" "$ACTION_TIMEOUT_MS"
}

if [[ -z "$GAME_URL" ]]; then
  STACK_BOOTSTRAPPED=1
  if (( FULL_GAMEPLAY == 1 )); then
    STACK_ARGS=(
      --allow-trusted-local-playtest
      --chain-enable
      --chain-local-standalone-test
      --chain-node-auto-attest-all
      --chain-link-policy shadow
      --major-world-event-visibility restricted
      --local-test-provider-authority "$LOCAL_PROVIDER_AUTHORITY"
      --local-test-provider-wasm "$LOCAL_PROVIDER_WASM"
      --local-test-provider-metadata "$LOCAL_PROVIDER_METADATA"
      --local-test-provider-agent-id starter-agent-0
      --local-test-provider-owner-binding local-test-owner-0
      --local-test-provider-finality-block-hash blake3:0000000000000000000000000000000000000000000000000000000000000000
      --local-test-provider-session-mode hosted_public_join
      "${STACK_ARGS[@]}"
    )
  elif (( STACK_CHAIN_ARG_EXPLICIT == 0 )); then
    # Hosted bootstrap defaults to a chain-disabled page-play lane.  Any
    # explicit --chain-* caller argument remains authoritative.
    STACK_ARGS+=(--chain-disable)
  fi
  if ((${#STACK_ARGS[@]} > 0)); then
    "$ROOT_DIR/scripts/run-launcher-stack.sh" \
      --with-llm \
      --agent-decision-source builtin_llm \
      --deployment-mode "$([[ "$FULL_GAMEPLAY" == "1" ]] && printf trusted_local_only || printf hosted_public_join)" \
      --json-ready \
      --run-id "$RUN_ID" \
      --output-dir "$OUT_DIR/runtime" \
      "${STACK_ARGS[@]}" >"$LAUNCH_LOG" 2>&1 &
  else
    "$ROOT_DIR/scripts/run-launcher-stack.sh" \
      --with-llm \
      --agent-decision-source builtin_llm \
      --deployment-mode "$([[ "$FULL_GAMEPLAY" == "1" ]] && printf trusted_local_only || printf hosted_public_join)" \
      --json-ready \
      --run-id "$RUN_ID" \
      --output-dir "$OUT_DIR/runtime" >"$LAUNCH_LOG" 2>&1 &
  fi
  LAUNCH_PID=$!
  if (( FULL_GAMEPLAY == 1 )); then
    wait_for_full_gameplay_readiness || exit 1
  fi
  deadline=$((SECONDS + STARTUP_TIMEOUT))
  while (( SECONDS < deadline )); do
    GAME_URL="$(python3 - "$LAUNCH_LOG" <<'PY'
import pathlib
import re
import sys
path = pathlib.Path(sys.argv[1])
if path.exists():
    for line in path.read_text(errors="replace").splitlines():
        match = re.search(r"^- URL: (https?://\S+)", line)
        if match:
            print(match.group(1))
            raise SystemExit
PY
)"
    [[ -n "$GAME_URL" ]] && break
    if ! kill -0 "$LAUNCH_PID" 2>/dev/null; then
      echo "error: launcher exited before publishing a URL" >&2
      exit 1
    fi
    sleep 1
  done
  [[ -n "$GAME_URL" ]] || { echo "error: launcher URL timeout" >&2; exit 1; }
  if (( TEST_LOGIN == 1 )); then
    GAME_URL="$(wait_for_authoritative_hosted_url)" || exit 1
    [[ -n "$GAME_URL" ]] || exit 1
  fi
fi

GAME_URL="$(python3 - "$GAME_URL" "$TEST_LOGIN" <<'PY'
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
import sys
parts = urlsplit(sys.argv[1])
query = dict(parse_qsl(parts.query, keep_blank_values=True))
query.update({"render_mode": "viewer", "test_api": "1"})
if sys.argv[2] == "1":
    # Explicit local test-login URL contract: hosted_test_login=1.
    query["hosted_test_login"] = "1"
print(urlunsplit((parts.scheme, parts.netloc, parts.path, urlencode(query), parts.fragment)))
PY
)"

require_loopback_url "$GAME_URL"
if (( HEADED == 1 )); then
  configure_loopback_browser_args
fi

ab_open "$SESSION" 1 "$GAME_URL"
wait_for_domcontentloaded
wait_for_webgl2
wait_for_cli_stage "test-api" wait --fn 'typeof window.__AW_TEST__ === "object"'
wait_for_cli_stage "test-login selector" wait --fn "Boolean(document.querySelector('[data-auth-action=\"test-login\"]'))"
run_visible_action "test-login action" click '[data-auth-action="test-login"]'
wait_for_js_true '(() => { const s = window.__AW_TEST__.getState(); const issued = s?.authRegistrationStatus === "issued" && s?.authRuntimeStatus === "issued"; const registered = s?.authRegistrationStatus === "registered" && ["registered", "registered_unbound"].includes(s?.authRuntimeStatus) && s?.authSessionEpoch != null; return s?.authReady === true && (issued || registered); })()' "hosted test-login auth readiness"
registration_required="$(ab_read_eval "$SESSION" '(() => { const s = window.__AW_TEST__.getState(); return s?.authRegistrationStatus === "issued" && s?.authRuntimeStatus === "issued"; })()' 2>/dev/null || true)"
case "$registration_required" in
  true|\"true\")
    register_hosted_player_session
    ;;
  false|\"false\")
    printf '[action:hosted player session registration action] skipped; test-login already registered the session\n' >>"$AB_LOG"
    ;;
  *)
    capture_failure_diagnostics "hosted player session registration decision"
    echo "error: hosted player session registration state was unavailable (phase: hosted player session registration decision)" >&2
    exit 1
    ;;
esac
wait_for_js_true '(() => { const s = window.__AW_TEST__.getState(); return s?.authReady === true && s?.authRegistrationStatus === "registered" && ["registered", "registered_unbound"].includes(s?.authRuntimeStatus) && s?.authSessionEpoch != null; })()' "hosted player session registration"

AGENT_ID_JSON="$(json_quote "$AGENT_ID")"
maybe_claim_first_agent "$AGENT_ID_JSON"
if (( FIRST_AGENT_CLAIM_PERFORMED == 0 )); then
  run_visible_action "exact agent selection action" click "$AGENT_SELECTOR"
  wait_for_js_true "(() => window.__AW_TEST__.getState()?.selectedId === ${AGENT_ID_JSON})()" "exact agent selection"
fi
maybe_claim_starter_oc "$AGENT_ID_JSON"
maybe_rebind_post_onboarding_session "$AGENT_ID_JSON"

wait_for_js_true "(() => { const s = window.__AW_TEST__.getState(); const p = s?.viewerProtocol || {}; return s?.authReady === true && s?.authRegistrationStatus === \"registered\" && [\"registered\", \"registered_unbound\"].includes(s?.authRuntimeStatus) && s?.authBoundAgentId === ${AGENT_ID_JSON} && s?.authSessionEpoch != null && s?.authBindingEpoch != null && p?.negotiated === true && Array.isArray(p?.capabilities) && p.capabilities.includes(\"prompt_control_result_v1\") && String(p?.authorityEpoch || \"\").length > 0; })()" "auth binding and prompt-result protocol readiness"

run_visible_action "command panel navigation action" click 'a[href="#viewer-details-panel"]'
wait_for_prompt_surface_continuity "$ACTION_TIMEOUT_MS"
run_visible_action "prompt overrides visibility action" click '[data-prompt-visibility-toggle="1"]'
wait_for_js_true '(() => { const s = window.__AW_TEST__.getState(); const panel = document.querySelector(`section#viewer-details-panel[data-viewer-route-panel="command"]`); return s?.promptOverridesVisible === true && Boolean(panel?.querySelector("details.command-surface__advanced-details > summary")) && Boolean(panel?.querySelector("#prompt-short")); })()' "prompt overrides visible state and exact prompt DOM"
run_visible_action "advanced prompt disclosure action" click 'details.command-surface__advanced-details > summary'
wait_for_js_true 'Boolean(document.querySelector("details.command-surface__advanced-details[open]"))' "advanced prompt disclosure"
wait_for_js_true 'Boolean(document.querySelector("#strong-auth-approval-code"))' "strong-auth approval input"
run_visible_action "strong-auth approval fill action" fill "#strong-auth-approval-code" "$OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE"
run_visible_action "short-term goal fill action" fill "#prompt-short" "$PROMPT_GOAL"

before_state="$(state_raw)"
write_safe_state "$before_state" "$OUT_DIR/state-before.json"
before_version="$(json_get "$before_state" selectedPromptVersion)"
if ! [[ "$before_version" =~ ^[0-9]+$ ]]; then
  echo "error: selected prompt version is unavailable before preview" >&2
  exit 1
fi

# All prompt changes below are visible browser actions. The test API is used
# only for the permitted server binding, state readback, and artifact capture.
run_visible_action "preview action" click 'button[data-prompt-action="preview"]'
wait_for_prompt_feedback preview
preview_state="$(state_raw)"
write_safe_state "$preview_state" "$OUT_DIR/state-after-preview.json"

run_visible_action "apply action" click 'button[data-prompt-action="apply"]'
wait_for_prompt_feedback apply
apply_state="$(state_raw)"
write_safe_state "$apply_state" "$OUT_DIR/state-after-apply.json"

run_visible_action "rollback target fill action" fill "#prompt-rollback-version" "$before_version"
run_visible_action "rollback action" click 'button[data-prompt-action="rollback"]'
wait_for_prompt_feedback rollback "$before_version"
rollback_state="$(state_raw)"
write_safe_state "$rollback_state" "$OUT_DIR/state-after-rollback.json"
ab_screenshot "$SESSION" "$OUT_DIR/prompt-control.png" >/dev/null
ab_cmd "$SESSION" snapshot >/dev/null 2>&1 || true

python3 - "$OUT_DIR" "$CASE_ID" "$AGENT_ID" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
summary = {
    "acceptanceEligible": True,
    "agentId": sys.argv[3],
    "blockedCases": {
        "AC-PROMPT-007": "requires a real second authorized actor revocation or transfer",
        "AC-PROMPT-009": "requires ordinary and race-path convergence with exactly one authority result",
        "AC-PROMPT-011": "requires reconnect and other-entry observation before persistence or sync claims",
    },
    "caseId": sys.argv[2],
    "evidenceTier": "real_browser_single_actor_strong_auth",
    "note": "Single-actor headed strong-auth preview/apply/rollback evidence. Race, gameplay consequence, reconnect, persistence, and sync claims remain out of scope.",
}
(root / "run-summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
write_manifest "$OUT_DIR" "real_browser_single_actor_strong_auth" true true "$AGENT_ID"
echo "headed single-actor strong-auth prompt flow complete (manifest: $OUT_DIR/artifact-manifest.json)"
exit 0
