#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
runner="$repo_root/scripts/viewer-prompt-control-regression.sh"
tmp_root=$(mktemp -d)
trap 'rm -rf "$tmp_root"' EXIT

test -x "$runner"

# RED-phase contract checks for the real visible strong-auth flow.  This
# optional narrow entrypoint intentionally runs before the contract-only
# artifact probe so missing runner behavior is reported directly, without
# launching a browser, game process, or model provider.
strong_auth_contract_failures=""
require_strong_auth_text() {
  local needle="$1"
  local label="$2"
  if ! rg -Fq -- "$needle" "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- ${label}: missing ${needle}"
  fi
}

require_strong_auth_regex() {
  local pattern="$1"
  local label="$2"
  if ! rg -q -- "$pattern" "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- ${label}: missing /${pattern}/"
  fi
}

run_full_gameplay_contract_checks() {
  local failures=""
  local required_text
  while IFS= read -r required_text; do
    if ! rg -Fq -- "$required_text" "$runner"; then
      failures="${failures}\n- missing ${required_text}"
    fi
  done <<'EOF'
--full-gameplay
--deployment-mode
trusted_local_only
--allow-trusted-local-playtest
--chain-enable
--chain-local-standalone-test
--chain-node-auto-attest-all
--chain-link-policy
shadow
--major-world-event-visibility
restricted
--local-test-provider-authority
local-test-provider-authority.json
--local-test-provider-wasm
.tmp/wasm-build-suite/module.runtime.local-test-provider.wasm
--local-test-provider-metadata
.tmp/wasm-build-suite/module.runtime.local-test-provider.metadata.json
--local-test-provider-agent-id
starter-agent-0
--local-test-provider-owner-binding
local-test-owner-0
--local-test-provider-finality-block-hash
blake3:0000000000000000000000000000000000000000000000000000000000000000
--local-test-provider-session-mode
hosted_public_join
LOCAL_TEST_PROVIDER_SETUP_ENABLED
STACK_READY
authority_grant
capability_invocation_context
local test provider artifact
EOF

  if rg -Fq -- 'PROVIDER_BOOTSTRAP_AUTHORITY_COUNT' "$runner"; then
    failures="${failures}\n- full-gameplay readiness must not use PROVIDER_BOOTSTRAP_AUTHORITY_COUNT"
  fi
  if ! rg -q -- '\[\[.*-f.*LOCAL_PROVIDER.*WASM|test -f.*LOCAL_PROVIDER.*WASM' "$runner"; then
    failures="${failures}\n- missing fail-closed local provider WASM artifact check"
  fi
  if ! rg -q -- '\[\[.*-f.*LOCAL_PROVIDER.*METADATA|test -f.*LOCAL_PROVIDER.*METADATA' "$runner"; then
    failures="${failures}\n- missing fail-closed local provider metadata artifact check"
  fi

  if [[ -n "$failures" ]]; then
    echo "viewer-prompt-control full-gameplay contract: RED (expected)" >&2
    printf '%b\n' "$failures" >&2
    return 1
  fi
  echo "viewer-prompt-control full-gameplay contract: passed"
}

run_strong_auth_contract_checks() {
  require_strong_auth_text '--test-login' 'explicit test-login option'
  require_strong_auth_text 'hosted_test_login=1' 'test-login query opt-in'
  require_strong_auth_text 'OASIS7_HOSTED_TEST_LOGIN_ENABLED' 'test-login environment gate'
  require_strong_auth_text 'loopback' 'loopback-only test-login gate'
  require_strong_auth_regex '127\\.0\\.0\\.1|localhost|\\[::1\\]' 'loopback host validation'

  require_strong_auth_text 'OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY' 'strong-auth public key environment'
  require_strong_auth_text 'OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY' 'strong-auth private key environment'
  require_strong_auth_text 'OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE' 'strong-auth approval environment'
  require_strong_auth_text "click '[data-auth-action=\"test-login\"]'" 'visible test-login action'
  require_strong_auth_regex 'data-agent-id.*AGENT_ID' 'exact AGENT_ID selection selector'
  if rg -Fq -- "click '[data-pixel-world-agent-marker=\"true\"][data-agent-id]'" "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- selection must not click an arbitrary agent marker"
  fi

  require_strong_auth_text 'wait --load domcontentloaded' 'DOM readiness wait'
  require_strong_auth_text 'document.readyState === "complete"' 'DOM readyState complete fallback'
  require_strong_auth_text 'document.readyState === "interactive"' 'DOM readyState interactive fallback'
  require_strong_auth_text 'domcontentloaded fallback' 'DOM fallback phase label'
  require_strong_auth_text 'capture_failure_diagnostics' 'readiness failure diagnostics'
  require_strong_auth_text 'agent-browser.log' 'agent-browser command diagnostics log'
  require_strong_auth_text 'snapshot' 'failure DOM snapshot diagnostics'
  require_strong_auth_text 'session info --json' 'failure browser session diagnostics'
  require_strong_auth_text 'tab list --json' 'failure browser tab diagnostics'
  require_strong_auth_text 'runtime/oasis7_viewer_live.log' 'runtime authoritative hosted URL source'
  require_strong_auth_text 'hosted_access' 'hosted access URL requirement'
  require_strong_auth_text 'authoritative hosted URL' 'fail-closed hosted URL diagnostic'
  require_strong_auth_text 'no-proxy-server' 'loopback headed browser no-proxy default'
  require_strong_auth_text 'Darwin' 'Darwin browser backend handling'
  require_strong_auth_text '--use-angle=gl' 'agent-browser library GL default handling'
  require_strong_auth_text '--use-angle=metal' 'Darwin WebGL2 Metal default'
  require_strong_auth_text 'getContext("webgl2")' 'WebGL2 readiness probe'
  require_strong_auth_text 'WebGL2 readiness' 'WebGL2 readiness phase label'
  require_strong_auth_text 'AGENT_ID="starter-agent-0"' 'starter agent default'
  require_strong_auth_text '--chain-disable' 'hosted bootstrap chain-disabled default'
  require_strong_auth_text 'chain argument' 'explicit chain argument override'
  require_strong_auth_text 'Claim First Agent' 'visible first-agent claim label'
  require_strong_auth_text '认领第一个 Agent' 'visible first-agent claim zh label'
  require_strong_auth_text 'a[href="#viewer-targets-panel"]' 'visible targets-panel navigation action'
  require_strong_auth_text '//*[@id="viewer-targets-panel"]//button' 'targets-panel scoped claim XPath'
  require_strong_auth_text 'normalize-space' 'normalized claim CTA text matching'
  if rg -n 'viewer-playthrough-action-claim-first-agent' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- claim must not depend on the stale data-testid selector"
  fi
  require_strong_auth_text 'Claim Your First OC' 'starter OC onboarding heading'
  require_strong_auth_text 'Claim Starter OC' 'starter OC onboarding CTA'
  require_strong_auth_text 'claim_starter_oc' 'starter OC gameplay authority action'
  require_strong_auth_text 'starter_oc_required_gate' 'starter OC onboarding overlay scope'
  require_strong_auth_text 'overlay dismissal' 'starter OC overlay dismissal phase'
  if rg -n 'viewer-playthrough-action-claim-starter-oc' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- starter OC claim must use scoped visible text/XPath, not a test id"
  fi
  require_strong_auth_text 'entityCounts' 'empty-world entity count gate'
  require_strong_auth_text 'lastGameplayActionFeedback' 'first-agent gameplay authority ack wait'
  require_strong_auth_text 'claim first agent action' 'first-agent claim action phase label'
  if rg -n '__AW_TEST__\.(claim|sendGameplayAction)' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- first-agent claim must use the visible button, not the test API"
  fi
  require_strong_auth_text 'run_visible_action' 'visible action failure wrapper'
  require_strong_auth_text 'test-login action' 'test-login action phase label'
  if rg -n 'ab_cmd "\$SESSION" (click|fill).*\/dev\/null' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- visible click/fill actions must not silently discard failures"
  fi
  require_strong_auth_text 'typeof window.__AW_TEST__ ===' 'test API readiness wait'
  require_strong_auth_text 'authReady' 'auth readiness wait'
  require_strong_auth_text 'authRegistrationStatus' 'auth registration wait'
  require_strong_auth_text 'authRuntimeStatus' 'runtime registration wait'
  require_strong_auth_text 'authBoundAgentId' 'agent binding wait'
  require_strong_auth_text 'authSessionEpoch' 'session epoch wait'
  require_strong_auth_text 'authBindingEpoch' 'binding epoch wait'
  require_strong_auth_text 'prompt_control_result_v1' 'prompt result protocol wait'
  require_strong_auth_text 'authorityEpoch' 'authority epoch wait'
  if rg -Fq -- 'wait --load networkidle' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- networkidle cannot be the sole long-lived Viewer readiness gate"
  fi

  require_strong_auth_text 'registerPlayerSessionForTest(null)' 'permitted hosted session-registration hook'
  require_strong_auth_text 'registerPlayerSessionForTest(${agent_id_json}, {forceRebind: true})' 'post-onboarding force-rebind hook'
  require_strong_auth_text 'post-onboarding auth binding rebind action' 'post-onboarding force-rebind diagnostics'
  require_strong_auth_text 'prompt surface page continuity' 'pre-prompt page-continuity guard'
  require_strong_auth_text 'details.command-surface__advanced-details > summary' 'exact advanced prompt disclosure DOM selector'
  require_strong_auth_text 'pre-prompt tab loss' 'distinct about:blank tab-loss diagnostic'
  if rg -Fq -- 'wait --text "Advanced Prompt Settings"' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- prompt readiness must use exact DOM state, not broad text wait"
  fi
  require_strong_auth_text 'hosted player session registration action' 'session-registration action phase label'
  require_strong_auth_text 'registered_unbound' 'unbound runtime registration wait'
  if rg -n 'ab_read_eval.*registerPlayerSessionForTest' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- hosted session registration must use non-retrying ab_eval"
  fi
  require_strong_auth_text 'fill "#strong-auth-approval-code"' 'visible approval-code input'
  require_strong_auth_text 'lastPromptFeedback' 'authority feedback readback'
  require_strong_auth_text 'strongAuthLastGrantActionId' 'strong-auth grant action readback'
  require_strong_auth_text 'prompt_control_preview' 'preview grant action'
  require_strong_auth_text 'prompt_control_apply' 'apply grant action'
  require_strong_auth_text 'prompt_control_rollback' 'rollback grant action'
  require_strong_auth_text 'accepted' 'preview authority result wait'
  require_strong_auth_text 'applied' 'apply authority result wait'
  require_strong_auth_text 'rolled_back_to_version' 'rollback authority result wait'
  require_strong_auth_text 'mutation_count' 'authority mutation-count assertion'
  require_strong_auth_text 'runtime_instance' 'runtime-only apply scope assertion'
  require_strong_auth_text 'persistence_scope' 'persistence scope assertion'
  require_strong_auth_text 'sync_scope' 'sync scope assertion'
  require_strong_auth_text 'fill "#prompt-rollback-version"' 'explicit rollback target'

  if [[ -n "$strong_auth_contract_failures" ]]; then
    echo "viewer-prompt-control strong-auth contract: RED (expected)" >&2
    printf '%b\n' "$strong_auth_contract_failures" >&2
    return 1
  fi
  echo "viewer-prompt-control strong-auth contract: passed"
}

if [[ "${1:-}" == "--strong-auth-contract-only" ]]; then
  run_strong_auth_contract_checks
  exit $?
fi

# PWT-004 gameplay settlement is an explicit runner lane.  Keep this source
# contract before all browser/launcher fixtures so a missing implementation
# fails deterministically without starting external processes.
run_full_gameplay_contract_checks

# Contract-only verification must never touch a browser or launcher/provider.
fake_bin="$tmp_root/bin"
mkdir -p "$fake_bin"
cat >"$fake_bin/agent-browser" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Minimal 0.37.1 lifecycle fixture.  The real runner owns the generated
# session and asks the CLI for diagnostics before closing that exact session;
# keep those commands observable without contacting a browser daemon.
if [[ "${1:-}" == "session" && "${2:-}" == "id" ]]; then
  printf '%s\n' "fixture-prompt-control-session"
  exit 0
fi
if [[ "${1:-}" == "session" && "${2:-}" == "info" ]]; then
  printf '%s\n' '{"session":"fixture-prompt-control-session","status":"open"}'
  exit 0
fi
if [[ "${1:-}" == "tab" && "${2:-}" == "list" ]]; then
  printf '%s\n' '[]'
  exit 0
fi
if [[ " $* " == *" open "* ]]; then
  case " $* " in
    *"hosted_test_login=1"*) ;;
    *)
      echo "fixture browser open URL missing hosted_test_login=1" >&2
      exit 1
      ;;
  esac
  if [[ "${VIEWER_PROMPT_FIXTURE_REQUIRE_METAL:-0}" == "1" && ( " $* " != *"--use-angle=metal"* || " $* " == *"--use-angle=gl"* ) ]]; then
    echo "fixture browser open args missing Darwin Metal backend" >&2
    exit 1
  fi
  if [[ "${VIEWER_PROMPT_FIXTURE_REQUIRE_EXPLICIT_GL:-0}" == "1" && ( " $* " != *"--use-angle=gl"* || " $* " == *"--use-angle=metal"* ) ]]; then
    echo "fixture browser open args did not preserve explicit GL backend" >&2
    exit 1
  fi
  exit 0
fi
if [[ "${VIEWER_PROMPT_FIXTURE_FAIL_DOM_WAIT:-0}" == "1" && " $* " == *" wait --load domcontentloaded"* ]]; then
  echo "fixture domcontentloaded wait timeout" >&2
  exit 17
fi
if [[ "${VIEWER_PROMPT_FIXTURE_FAIL_CLAIM_BUTTON_WAIT:-0}" == "1" && ( "$*" == *normalize-space* || "$*" == *claim-first-agent* ) ]]; then
  echo "fixture Claim First Agent button wait failed" >&2
  exit 23
fi
if [[ "${1:-}" == "click" && "${2:-}" == 'a[href="#viewer-targets-panel"]' && -n "${VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT:-}" ]]; then
  panel_nav_count=0
  if [[ -f "$VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT" ]]; then
    panel_nav_count=$(<"$VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT")
  fi
  printf '%s\n' "$((panel_nav_count + 1))" >"$VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT"
  exit 0
fi
if [[ "${1:-}" == "click" && "${2:-}" == *'viewer-targets-panel'* && "${2:-}" == *'normalize-space'* && -n "${VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT:-}" ]]; then
  claim_action_count=0
  if [[ -f "$VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT" ]]; then
    claim_action_count=$(<"$VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT")
  fi
  printf '%s\n' "$((claim_action_count + 1))" >"$VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT"
  exit 0
fi
if [[ "${1:-}" == "click" && "${2:-}" == *'starter_oc_required_gate'* && "${2:-}" == *'normalize-space'* && -n "${VIEWER_PROMPT_FIXTURE_STARTER_OC_ACTION_COUNT:-}" ]]; then
  starter_oc_action_count=0
  if [[ -f "$VIEWER_PROMPT_FIXTURE_STARTER_OC_ACTION_COUNT" ]]; then
    starter_oc_action_count=$(<"$VIEWER_PROMPT_FIXTURE_STARTER_OC_ACTION_COUNT")
  fi
  printf '%s\n' "$((starter_oc_action_count + 1))" >"$VIEWER_PROMPT_FIXTURE_STARTER_OC_ACTION_COUNT"
  exit 0
fi
if [[ "${1:-}" == "click" && "${2:-}" == '[data-auth-action="test-login"]' && "${VIEWER_PROMPT_FIXTURE_FAIL_ACTION:-}" == "test-login" ]]; then
  action_count=0
  if [[ -n "${VIEWER_PROMPT_FIXTURE_ACTION_COUNT:-}" && -f "$VIEWER_PROMPT_FIXTURE_ACTION_COUNT" ]]; then
    action_count=$(<"$VIEWER_PROMPT_FIXTURE_ACTION_COUNT")
  fi
  if [[ -n "${VIEWER_PROMPT_FIXTURE_ACTION_COUNT:-}" ]]; then
    printf '%s\n' "$((action_count + 1))" >"$VIEWER_PROMPT_FIXTURE_ACTION_COUNT"
  fi
  echo "fixture test-login action failure" >&2
  exit 19
fi
if [[ "${1:-}" == "eval" ]]; then
  script=$(cat)
  if [[ "$script" == *'document.readyState === "complete"'* && "$script" == *'document.readyState === "interactive"'* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_FALLBACK_MARKER:-}" ]]; then
      : >"$VIEWER_PROMPT_FIXTURE_FALLBACK_MARKER"
    fi
    printf '%s\n' 'true'
  elif [[ "$script" == *'prompt_surface_missing:'* && "$script" == *'details.command-surface__advanced-details > summary'* ]]; then
    if [[ "${VIEWER_PROMPT_FIXTURE_PRE_PROMPT_TAB_LOST:-0}" == "1" ]]; then
      printf '%s\n' '"tab_lost"'
    elif [[ "${VIEWER_PROMPT_FIXTURE_PRE_PROMPT_UI_MISSING:-0}" == "1" ]]; then
      printf '%s\n' '"prompt_surface_missing:http://127.0.0.1:9/"'
    else
      printf '%s\n' '"ready"'
    fi
  elif [[ "$script" == *'registerPlayerSessionForTest(null)'* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT:-}" ]]; then
      registration_count=0
      if [[ -f "$VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT" ]]; then
        registration_count=$(<"$VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT")
      fi
      printf '%s\n' "$((registration_count + 1))" >"$VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT"
    fi
    if [[ "${VIEWER_PROMPT_FIXTURE_FAIL_SESSION_REGISTRATION:-0}" == "1" ]]; then
      echo "fixture hosted player session registration failure" >&2
      exit 29
    fi
    printf '%s\n' 'true'
  elif [[ "$script" == *'registerPlayerSessionForTest('* && "$script" == *'{forceRebind: true}'* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_FORCE_REBIND_COUNT:-}" ]]; then
      force_rebind_count=0
      if [[ -f "$VIEWER_PROMPT_FIXTURE_FORCE_REBIND_COUNT" ]]; then
        force_rebind_count=$(<"$VIEWER_PROMPT_FIXTURE_FORCE_REBIND_COUNT")
      fi
      printf '%s\n' "$((force_rebind_count + 1))" >"$VIEWER_PROMPT_FIXTURE_FORCE_REBIND_COUNT"
    fi
    printf '%s\n' 'true'
  elif [[ "$script" == *'authBoundAgentId'* && "$script" == *'authBindingEpoch == null'* ]]; then
    if [[ "${VIEWER_PROMPT_FIXTURE_BINDING_EPOCH_MISSING:-0}" == "1" ]]; then
      printf '%s\n' 'true'
    else
      printf '%s\n' 'false'
    fi
  elif [[ "$script" == '(() => { const s = window.__AW_TEST__.getState(); return s?.authRegistrationStatus === "issued" && s?.authRuntimeStatus === "issued"; })()' ]]; then
    if [[ "${VIEWER_PROMPT_FIXTURE_ALREADY_REGISTERED:-0}" == "1" ]]; then
      printf '%s\n' 'false'
    else
      printf '%s\n' 'true'
    fi
  elif [[ "$script" == *'starter_oc_required_gate'* && "$script" == *'Claim Starter OC'* ]]; then
    if [[ "${VIEWER_PROMPT_FIXTURE_STARTER_OC_ONBOARDING:-0}" == "1" ]]; then
      printf '%s\n' 'true'
    else
      printf '%s\n' 'false'
    fi
  elif [[ "$script" == 'window.__AW_TEST__.getState()' ]]; then
    printf '%s\n' '{"authReady":true,"authRegistrationStatus":"registered","authRuntimeStatus":"registered","authBoundAgentId":"starter-agent-0","authSessionEpoch":1,"authBindingEpoch":1,"viewerProtocol":{"negotiated":true,"capabilities":["prompt_control_result_v1"],"authorityEpoch":"fixture-authority"},"selectedId":"starter-agent-0","selectedPromptVersion":0,"lastPromptFeedback":null,"strongAuthLastGrantActionId":null,"strongAuthLastGrantError":null}'
  elif [[ "$script" == *"entityCounts"* && "$script" == *"document.querySelector"* ]]; then
    if [[ "${VIEWER_PROMPT_FIXTURE_EMPTY_WORLD:-0}" == "1" ]]; then
      printf '%s\n' 'true'
    else
      printf '%s\n' 'false'
    fi
  elif [[ "$script" == *"lastGameplayActionFeedback"* && "$script" == *"claim_starter_oc"* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_STARTER_OC_ACK_MARKER:-}" ]]; then
      : >"$VIEWER_PROMPT_FIXTURE_STARTER_OC_ACK_MARKER"
    fi
    printf '%s\n' 'true'
  elif [[ "$script" == *"lastGameplayActionFeedback"* && "$script" == *"claim_first_agent"* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_CLAIM_ACK_MARKER:-}" ]]; then
      : >"$VIEWER_PROMPT_FIXTURE_CLAIM_ACK_MARKER"
    fi
    printf '%s\n' 'true'
  else
    printf '%s\n' 'true'
  fi
  exit 0
fi
if [[ "${1:-}" == "record" && "${2:-}" == "stop" ]]; then
  exit 0
fi
if [[ "${1:-}" == "close" ]]; then
  exit 0
fi
# All other calls are harmless no-ops for this blocked-status launch probe.
exit 0
EOF
chmod +x "$fake_bin/agent-browser"

run_contract() {
  local out_dir="$1"
  PATH="$fake_bin:$PATH" "$runner" \
    --contract-only \
    --headed \
    --case-id PWT-004 \
    --out-dir "$out_dir"
}

first_out="$tmp_root/first"
second_out="$tmp_root/second"
run_contract "$first_out"
run_contract "$second_out"

fallback_out="$tmp_root/domcontentloaded-fallback"
fallback_marker="$tmp_root/domcontentloaded-fallback-seen"
VIEWER_PROMPT_FIXTURE_FAIL_DOM_WAIT=1 \
  VIEWER_PROMPT_FIXTURE_FALLBACK_MARKER="$fallback_marker" \
  VIEWER_PROMPT_FIXTURE_REQUIRE_METAL=1 \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$fallback_out"
test -f "$fallback_out/agent-browser.log"
test -f "$fallback_marker"
rg -Fq 'domcontentloaded fallback' "$fallback_out/agent-browser.log"
test -f "$fallback_out/failure-domcontentloaded-session-info.json"
test -f "$fallback_out/failure-domcontentloaded-tabs.json"

action_failure_out="$tmp_root/action-failure"
action_count_file="$tmp_root/test-login-action-count"
set +e
VIEWER_PROMPT_FIXTURE_FAIL_ACTION=test-login \
  VIEWER_PROMPT_FIXTURE_ACTION_COUNT="$action_count_file" \
  VIEWER_PROMPT_FIXTURE_REQUIRE_EXPLICIT_GL=1 \
  AGENT_BROWSER_ARGS='--use-angle=gl' \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$action_failure_out"
action_failure_rc=$?
set -e
test "$action_failure_rc" -ne 0
test -f "$action_count_file"
test "$(<"$action_count_file")" = 1
rg -Fq '[action:test-login action] command failed' "$action_failure_out/agent-browser.log"
test -f "$action_failure_out/failure-test-login_action-session-info.json"

claim_out="$tmp_root/claim-first-agent"
claim_count_file="$tmp_root/claim-first-agent-count"
panel_nav_count_file="$tmp_root/claim-first-agent-panel-nav-count"
session_registration_count_file="$tmp_root/claim-first-agent-session-registration-count"
claim_ack_marker="$tmp_root/claim-first-agent-ack-seen"
starter_oc_action_count_file="$tmp_root/claim-starter-oc-count"
starter_oc_ack_marker="$tmp_root/claim-starter-oc-ack-seen"
VIEWER_PROMPT_FIXTURE_EMPTY_WORLD=1 \
  VIEWER_PROMPT_FIXTURE_STARTER_OC_ONBOARDING=1 \
  VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT="$claim_count_file" \
  VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT="$panel_nav_count_file" \
  VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT="$session_registration_count_file" \
  VIEWER_PROMPT_FIXTURE_STARTER_OC_ACTION_COUNT="$starter_oc_action_count_file" \
  VIEWER_PROMPT_FIXTURE_STARTER_OC_ACK_MARKER="$starter_oc_ack_marker" \
  VIEWER_PROMPT_FIXTURE_CLAIM_ACK_MARKER="$claim_ack_marker" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$claim_out"
test -f "$claim_count_file"
test "$(<"$claim_count_file")" = 1
test -f "$panel_nav_count_file"
test "$(<"$panel_nav_count_file")" = 1
test -f "$session_registration_count_file"
test "$(<"$session_registration_count_file")" = 1
test -f "$starter_oc_action_count_file"
test "$(<"$starter_oc_action_count_file")" = 1
test -f "$starter_oc_ack_marker"
test -f "$claim_ack_marker"
rg -Fq '[action:claim first agent action]' "$claim_out/agent-browser.log"

# Gameplay onboarding may bind the claimed agent before publishing a binding
# epoch.  The runner must repair that precise state once, through the existing
# non-retrying session registration hook, then resume protocol readiness.
force_rebind_out="$tmp_root/post-onboarding-force-rebind"
force_rebind_count="$tmp_root/post-onboarding-force-rebind-count"
VIEWER_PROMPT_FIXTURE_ALREADY_REGISTERED=1 \
  VIEWER_PROMPT_FIXTURE_BINDING_EPOCH_MISSING=1 \
  VIEWER_PROMPT_FIXTURE_FORCE_REBIND_COUNT="$force_rebind_count" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$force_rebind_out"
test -f "$force_rebind_count"
test "$(<"$force_rebind_count")" = 1
rg -Fq '[action:post-onboarding auth binding rebind action] completed' "$force_rebind_out/agent-browser.log"

force_rebind_skip_out="$tmp_root/post-onboarding-force-rebind-skip"
force_rebind_skip_count="$tmp_root/post-onboarding-force-rebind-skip-count"
VIEWER_PROMPT_FIXTURE_ALREADY_REGISTERED=1 \
  VIEWER_PROMPT_FIXTURE_FORCE_REBIND_COUNT="$force_rebind_skip_count" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$force_rebind_skip_out"
if [[ -f "$force_rebind_skip_count" ]]; then
  test "$(<"$force_rebind_skip_count")" = 0
fi
rg -Fq '[action:post-onboarding auth binding rebind action] skipped; binding epoch already present or agent not bound' "$force_rebind_skip_out/agent-browser.log"

# A lost browser tab must fail before any prompt action and be distinguishable
# from a live page whose exact prompt DOM simply did not become ready.
tab_loss_out="$tmp_root/pre-prompt-tab-loss"
set +e
VIEWER_PROMPT_FIXTURE_ALREADY_REGISTERED=1 \
  VIEWER_PROMPT_FIXTURE_PRE_PROMPT_TAB_LOST=1 \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$tab_loss_out"
tab_loss_rc=$?
set -e
test "$tab_loss_rc" -ne 0
rg -Fq '[prompt surface page continuity] tab lost to about:blank' "$tab_loss_out/agent-browser.log"
test -f "$tab_loss_out/failure-pre-prompt_tab_loss-tabs.json"
if rg -Fq '[action:advanced prompt disclosure action]' "$tab_loss_out/agent-browser.log"; then
  echo "advanced prompt action ran after pre-prompt tab loss" >&2
  exit 1
fi

# The visible test-login action may finish server-side registration before the
# runner observes its first auth state.  In that case the registration hook
# must not be invoked a second time.
already_registered_out="$tmp_root/already-registered-login"
already_registered_count="$tmp_root/already-registered-session-registration-count"
VIEWER_PROMPT_FIXTURE_ALREADY_REGISTERED=1 \
  VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT="$already_registered_count" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$already_registered_out"
if [[ -f "$already_registered_count" ]]; then
  test "$(<"$already_registered_count")" = 0
fi

claim_fail_out="$tmp_root/claim-first-agent-failure"
claim_fail_count="$tmp_root/claim-first-agent-failure-count"
claim_fail_panel_nav_count="$tmp_root/claim-first-agent-failure-panel-nav-count"
set +e
VIEWER_PROMPT_FIXTURE_EMPTY_WORLD=1 \
  VIEWER_PROMPT_FIXTURE_FAIL_CLAIM_BUTTON_WAIT=1 \
  VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT="$claim_fail_count" \
  VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT="$claim_fail_panel_nav_count" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$claim_fail_out"
claim_fail_rc=$?
set -e
test "$claim_fail_rc" -ne 0
if [[ -f "$claim_fail_count" ]]; then
  test "$(<"$claim_fail_count")" = 0
fi
test -f "$claim_fail_panel_nav_count"
test "$(<"$claim_fail_panel_nav_count")" = 1

registration_fail_out="$tmp_root/session-registration-failure"
registration_fail_count="$tmp_root/session-registration-failure-count"
registration_fail_panel_nav_count="$tmp_root/session-registration-failure-panel-nav-count"
set +e
VIEWER_PROMPT_FIXTURE_FAIL_SESSION_REGISTRATION=1 \
  VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT="$registration_fail_count" \
  VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT="$registration_fail_panel_nav_count" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$registration_fail_out"
registration_fail_rc=$?
set -e
test "$registration_fail_rc" -ne 0
test -f "$registration_fail_count"
test "$(<"$registration_fail_count")" = 1
if [[ -f "$registration_fail_panel_nav_count" ]]; then
  test "$(<"$registration_fail_panel_nav_count")" = 0
fi
test -f "$registration_fail_out/failure-hosted_player_session_registration_action-session-info.json"
test -f "$claim_fail_out/failure-claim_first_agent_button-session-info.json"

first_manifest=$(find "$first_out" -name artifact-manifest.json -type f -print -quit)
second_manifest=$(find "$second_out" -name artifact-manifest.json -type f -print -quit)
test -n "$first_manifest"
test -n "$second_manifest"
cmp -s "$first_manifest" "$second_manifest"

python3 - "$first_manifest" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest["schema"] == "oasis7.viewer.prompt-control-artifact-manifest/v1"
assert manifest["caseId"] == "PWT-004"
assert manifest["evidenceTier"] == "contract_only"
assert manifest["acceptanceEligible"] is False
assert manifest["providerCallsDuringVerification"] is False
assert manifest["browserMode"] == "headed"
assert manifest["visibleActionContract"] == {
    "selection": "[data-pixel-world-agent-marker=\\\"true\\\"][data-agent-id=\\\"${AGENT_ID}\\\"]",
    "promptDisclosure": "Advanced Prompt Settings",
    "shortTermGoal": "#prompt-short",
    "testLogin": "[data-auth-action=\\\"test-login\\\"]",
    "approvalCode": "#strong-auth-approval-code",
    "preview": "button[data-prompt-action=\\\"preview\\\"]",
    "apply": "button[data-prompt-action=\\\"apply\\\"]",
    "rollbackTarget": "#prompt-rollback-version",
    "rollback": "button[data-prompt-action=\\\"rollback\\\"]",
}
paths = [entry["path"] for entry in manifest["artifacts"]]
assert paths == sorted(paths)
assert paths == ["contract.json", "manifest-input.json"]
for entry in manifest["artifacts"]:
    assert set(entry) == {"bytes", "path", "sha256"}
    assert len(entry["sha256"]) == 64
PY

if "$runner" --headless --contract-only --out-dir "$tmp_root/headless" >"$tmp_root/headless.log" 2>&1; then
  echo "headless contract unexpectedly passed" >&2
  exit 1
fi
grep -Fq -- "--headed is required" "$tmp_root/headless.log"

if rg -n 'sendPromptControl|__AW_TEST__\.sendPromptControl' "$runner"; then
  echo "runner must not replace visible prompt actions with sendPromptControl" >&2
  exit 1
fi
rg -Fq 'data-pixel-world-agent-marker' "$runner"
rg -Fq 'fill "#prompt-short"' "$runner"
rg -Fq 'button[data-prompt-action="preview"]' "$runner"
rg -Fq 'button[data-prompt-action="apply"]' "$runner"
rg -Fq 'button[data-prompt-action="rollback"]' "$runner"

# macOS ships Bash 3.2, where set -u expands an empty array as unbound.
# Exercise the real launch branch in a sandbox so no browser or provider is
# contacted while checking that zero passthrough arguments remain valid.
if /bin/bash -c 'set -u; empty=(); : "${empty[@]}"' >/dev/null 2>&1; then
  echo "Bash 3 empty-array compatibility probe unavailable; using source guard"
else
  sandbox="$tmp_root/bash3-sandbox"
  mkdir -p "$sandbox/scripts" "$sandbox/bin"
  cp "$runner" "$sandbox/scripts/viewer-prompt-control-regression.sh"
  ln -s "$repo_root/scripts/agent-browser-lib.sh" "$sandbox/scripts/agent-browser-lib.sh"
  ln -s "$repo_root/scripts/viewer-web-dist-contract.sh" "$sandbox/scripts/viewer-web-dist-contract.sh"
cat >"$sandbox/scripts/run-launcher-stack.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY:-}" == "fixture" ]]
[[ "${OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY:-}" == "fixture" ]]
[[ "${OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE:-}" == "fixture" ]]
[[ "${OASIS7_HOSTED_TEST_LOGIN_ENABLED:-}" == "1" ]]
output_dir=""
chain_disable_seen=0
while (($# > 0)); do
  if [[ "${1:-}" == "--output-dir" ]]; then
    output_dir="${2:?missing output dir}"
    shift 2
  elif [[ "${1:-}" == "--chain-disable" ]]; then
    chain_disable_seen=1
    shift
  else
    shift
  fi
done
[[ "$chain_disable_seen" == "1" ]]
mkdir -p "$output_dir"
printf '%s\n' '- URL: http://127.0.0.1:9'
printf '%s\n' '- URL: http://127.0.0.1:9/?render_mode=viewer&ws=ws%3A%2F%2F127.0.0.1%3A11&hosted_access=fixture-authority' \
  >"$output_dir/oasis7_viewer_live.log"
EOF
  cat >"$sandbox/bin/agent-browser" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ " $* " == *" session id "* ]]; then
  printf '%s\n' 'viewer-prompt-control-test-session'
elif [[ " $* " == *" session info "* || " $* " == *" tab list "* ]]; then
  printf '%s\n' '{"success":true,"data":{}}'
elif [[ " $* " == *" open "* ]]; then
  case " $* " in
    *"hosted_test_login=1"*) ;;
    *) exit 1 ;;
  esac
  if [[ "${VIEWER_PROMPT_FIXTURE_REQUIRE_HOSTED_ACCESS:-0}" == "1" && " $* " != *"hosted_access=fixture-authority"* ]]; then
    echo "fixture browser open URL missing authoritative hosted_access" >&2
    exit 1
  fi
  if [[ "${VIEWER_PROMPT_FIXTURE_REQUIRE_NO_PROXY:-0}" == "1" && " $* " != *"--no-proxy-server"* ]]; then
    echo "fixture browser open args missing --no-proxy-server" >&2
    exit 1
  fi
  if [[ "${VIEWER_PROMPT_FIXTURE_REQUIRE_METAL:-0}" == "1" && ( " $* " != *"--use-angle=metal"* || " $* " == *"--use-angle=gl"* ) ]]; then
    echo "fixture browser open args missing Darwin Metal backend" >&2
    exit 1
  fi
elif [[ "${1:-}" == "eval" && "${2:-}" == "--stdin" ]]; then
  script=$(cat)
  if [[ "$script" == 'window.__AW_TEST__.getState()' ]]; then
    printf '%s\n' '{"authReady":true,"authRegistrationStatus":"registered","authRuntimeStatus":"registered","authBoundAgentId":"agent-1","authSessionEpoch":1,"authBindingEpoch":1,"viewerProtocol":{"negotiated":true,"capabilities":["prompt_control_result_v1"],"authorityEpoch":"fixture-authority"},"selectedId":"agent-1","selectedPromptVersion":0,"lastPromptFeedback":null,"strongAuthLastGrantActionId":null,"strongAuthLastGrantError":null}'
  elif [[ "$script" == *'prompt_surface_missing:'* && "$script" == *'details.command-surface__advanced-details > summary'* ]]; then
    printf '%s\n' '"ready"'
  else
    printf '%s\n' 'true'
  fi
fi
exit 0
EOF
  chmod +x "$sandbox/scripts/run-launcher-stack.sh" "$sandbox/bin/agent-browser"
  set +e
  VIEWER_PROMPT_FIXTURE_REQUIRE_HOSTED_ACCESS=1 \
    VIEWER_PROMPT_FIXTURE_REQUIRE_NO_PROXY=1 \
    VIEWER_PROMPT_FIXTURE_REQUIRE_METAL=1 \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
    OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
    OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
    PATH="$sandbox/bin:$PATH" /bin/bash "$sandbox/scripts/viewer-prompt-control-regression.sh" \
    --headed --test-login --out-dir "$tmp_root/bash3-run"
  result_code=$?
  set -e
  if [[ "$result_code" -eq 0 ]]; then
    echo "Bash 3 empty-array compatibility flow completed with expected green status"
  else
    echo "Bash 3 empty-array compatibility flow failed: $result_code" >&2
    exit 1
  fi
fi
rg -Fq 'if ((${#STACK_ARGS[@]} > 0)); then' "$runner"

echo "viewer-prompt-control runner contract: passed"
