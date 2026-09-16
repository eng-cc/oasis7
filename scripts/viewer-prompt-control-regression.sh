#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASE_ID="PWT-004"
OUT_DIR="output/playwright/prompt-control/${CASE_ID}"
HEADED=0
CONTRACT_ONLY=0
GAME_URL=""
AGENT_ID="agent-1"
PROMPT_GOAL="Inspect the selected agent's prompt control state."
STARTUP_TIMEOUT=180
ACTION_TIMEOUT_MS=10000
TEST_LOGIN=0
STACK_ARGS=()

usage() {
  cat <<'EOF'
Usage: viewer-prompt-control-regression.sh --headed [options]

Runs the PWT-004 prompt-control flow in a headed browser.  --contract-only
produces the deterministic runner contract and manifest without launching a
browser, game process, or model provider.

Options:
  --headed | --headless
  --contract-only
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
  echo "error: timed out waiting for ${label}" >&2
  return 1
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
  if ((${#STACK_ARGS[@]} > 0)); then
    "$ROOT_DIR/scripts/run-launcher-stack.sh" \
      --with-llm \
      --agent-decision-source builtin_llm \
      --deployment-mode hosted_public_join \
      --json-ready \
      --run-id "$RUN_ID" \
      --output-dir "$OUT_DIR/runtime" \
      "${STACK_ARGS[@]}" >"$LAUNCH_LOG" 2>&1 &
  else
    "$ROOT_DIR/scripts/run-launcher-stack.sh" \
      --with-llm \
      --agent-decision-source builtin_llm \
      --deployment-mode hosted_public_join \
      --json-ready \
      --run-id "$RUN_ID" \
      --output-dir "$OUT_DIR/runtime" >"$LAUNCH_LOG" 2>&1 &
  fi
  LAUNCH_PID=$!
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

ab_open "$SESSION" 1 "$GAME_URL"
ab_read_retry "$SESSION" wait --load domcontentloaded >/dev/null
ab_read_retry "$SESSION" wait --fn 'typeof window.__AW_TEST__ === "object"' >/dev/null
ab_read_retry "$SESSION" wait --fn "Boolean(document.querySelector('[data-auth-action=\"test-login\"]'))" >/dev/null
ab_cmd "$SESSION" click '[data-auth-action="test-login"]' >/dev/null 2>&1
wait_for_js_true '(() => { const s = window.__AW_TEST__.getState(); return s?.authReady === true && s?.authRegistrationStatus === "issued" && s?.authRuntimeStatus === "issued"; })()' "hosted test-login auth issuance"

AGENT_ID_JSON="$(json_quote "$AGENT_ID")"
ab_cmd "$SESSION" click "$AGENT_SELECTOR" >/dev/null 2>&1
wait_for_js_true "(() => window.__AW_TEST__.getState()?.selectedId === ${AGENT_ID_JSON})()" "exact agent selection"

# This is the permitted server-binding hook. It does not select, type, submit,
# or replace any player-facing prompt action.
ab_eval "$SESSION" "window.__AW_TEST__.registerPlayerSessionForTest(${AGENT_ID_JSON})" >/dev/null 2>&1
wait_for_js_true "(() => { const s = window.__AW_TEST__.getState(); const p = s?.viewerProtocol || {}; return s?.authReady === true && s?.authRegistrationStatus === \"registered\" && [\"registered\", \"registered_unbound\"].includes(s?.authRuntimeStatus) && s?.authBoundAgentId === ${AGENT_ID_JSON} && s?.authSessionEpoch != null && s?.authBindingEpoch != null && p?.negotiated === true && Array.isArray(p?.capabilities) && p.capabilities.includes(\"prompt_control_result_v1\") && String(p?.authorityEpoch || \"\").length > 0; })()" "auth binding and prompt-result protocol readiness"

AGENT_BROWSER_DEFAULT_TIMEOUT="$ACTION_TIMEOUT_MS" \
  ab_read_retry "$SESSION" wait --text "Advanced Prompt Settings" >/dev/null
ab_cmd "$SESSION" click 'details.command-surface__advanced-details > summary' >/dev/null 2>&1
wait_for_js_true 'Boolean(document.querySelector("details.command-surface__advanced-details[open]"))' "advanced prompt disclosure"
wait_for_js_true 'Boolean(document.querySelector("#strong-auth-approval-code"))' "strong-auth approval input"
ab_cmd "$SESSION" fill "#strong-auth-approval-code" "$OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE" >/dev/null 2>&1
ab_cmd "$SESSION" fill "#prompt-short" "$PROMPT_GOAL" >/dev/null 2>&1

before_state="$(state_raw)"
write_safe_state "$before_state" "$OUT_DIR/state-before.json"
before_version="$(json_get "$before_state" selectedPromptVersion)"
if ! [[ "$before_version" =~ ^[0-9]+$ ]]; then
  echo "error: selected prompt version is unavailable before preview" >&2
  exit 1
fi

# All prompt changes below are visible browser actions. The test API is used
# only for the permitted server binding, state readback, and artifact capture.
ab_cmd "$SESSION" click 'button[data-prompt-action="preview"]' >/dev/null 2>&1
wait_for_prompt_feedback preview
preview_state="$(state_raw)"
write_safe_state "$preview_state" "$OUT_DIR/state-after-preview.json"

ab_cmd "$SESSION" click 'button[data-prompt-action="apply"]' >/dev/null 2>&1
wait_for_prompt_feedback apply
apply_state="$(state_raw)"
write_safe_state "$apply_state" "$OUT_DIR/state-after-apply.json"

ab_cmd "$SESSION" fill "#prompt-rollback-version" "$before_version" >/dev/null 2>&1
ab_cmd "$SESSION" click 'button[data-prompt-action="rollback"]' >/dev/null 2>&1
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
