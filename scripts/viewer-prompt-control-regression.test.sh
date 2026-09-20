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
  if ! grep -Fq -- "$needle" "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- ${label}: missing ${needle}"
  fi
}

require_strong_auth_regex() {
  local pattern="$1"
  local label="$2"
  if ! grep -Eq -- "$pattern" "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- ${label}: missing /${pattern}/"
  fi
}

run_full_gameplay_contract_checks() {
  local failures=""
  local required_text
  while IFS= read -r required_text; do
    if ! grep -Fq -- "$required_text" "$runner"; then
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
.tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.wasm
--local-test-provider-metadata
.tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.metadata.json
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

  if grep -Fq -- 'PROVIDER_BOOTSTRAP_AUTHORITY_COUNT' "$runner"; then
    failures="${failures}\n- full-gameplay readiness must not use PROVIDER_BOOTSTRAP_AUTHORITY_COUNT"
  fi
  if ! grep -Eq -- '\[\[.*-f.*LOCAL_PROVIDER.*WASM|test -f.*LOCAL_PROVIDER.*WASM' "$runner"; then
    failures="${failures}\n- missing fail-closed local provider WASM artifact check"
  fi
  if ! grep -Eq -- '\[\[.*-f.*LOCAL_PROVIDER.*METADATA|test -f.*LOCAL_PROVIDER.*METADATA' "$runner"; then
    failures="${failures}\n- missing fail-closed local provider metadata artifact check"
  fi

  if [[ -n "$failures" ]]; then
    echo "viewer-prompt-control full-gameplay contract: RED (expected)" >&2
    printf '%b\n' "$failures" >&2
    return 1
  fi
  echo "viewer-prompt-control full-gameplay contract: passed"
}

run_hosted_local_mock_contract_checks() {
  local failures=""
  local required_text
  while IFS= read -r required_text; do
    if ! grep -Fq -- "$required_text" "$runner"; then
      failures="${failures}\n- missing ${required_text}"
    fi
  done <<'EOF'
--hosted-local-mock
--test-tier-required
test_tier_required
hosted_local_mock_seeded_agent
--source-base
binary-provenance.json
oasis7.viewer.binary-provenance/v1
source tree is dirty; refusing exact-head binary provenance
refusing to reuse existing provenance output
test-tier-required-build.json
test-tier-required-build.log
capture_success_browser_diagnostics
browser-console.log
browser-errors.log
browserDiagnostics
quiesce_for_manifest
quiescence
sha256
rustcVersion
cargoVersion
activeToolchain
profile
source_head
source_base
--chain-enable
--chain-local-standalone-test
--local-test-provider-authority
local-test-provider-authority.json
--local-test-provider-wasm
.tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.wasm
--local-test-provider-metadata
.tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.metadata.json
--local-test-provider-agent-id
AGENT_ID
--local-test-provider-owner-binding
local-test-owner-0
--local-test-provider-finality-block-hash
blake3:0000000000000000000000000000000000000000000000000000000000000000
--local-test-provider-session-mode
hosted_public_join
provider_backed
provider_local_mock
worldsim_provider_v1
loopback_http
player_parity
--skip-llm-provider-preflight
--no-auto-play
OASIS7_RUN_LAUNCHER_STACK_SKIP_SOURCE_BUILD=1
seed-42
EOF

  if grep -Fq -- '--hosted-local-mock)' "$runner" && ! grep -Fq -- 'requires explicit --test-tier-required' "$runner"; then
    failures="${failures}\n- Hosted local-mock route must require explicit test_tier_required"
  fi
  if ! grep -Fq -- '--chain-enable' "$runner" || ! grep -Fq -- '--chain-local-standalone-test' "$runner"; then
    failures="${failures}\n- Hosted local-mock route must use the explicit DevLocal standalone chain"
  fi
  if [[ -n "$failures" ]]; then
    echo "viewer-prompt-control Hosted local-mock contract: RED" >&2
    printf '%b\n' "$failures" >&2
    return 1
  fi
  echo "viewer-prompt-control Hosted local-mock contract: passed"
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
  if grep -Fq -- "click '[data-pixel-world-agent-marker=\"true\"][data-agent-id]'" "$runner"; then
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
  if grep -En 'viewer-playthrough-action-claim-first-agent' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- claim must not depend on the stale data-testid selector"
  fi
  require_strong_auth_text 'Claim Your First OC' 'starter OC onboarding heading'
  require_strong_auth_text 'Claim Starter OC' 'starter OC onboarding CTA'
  require_strong_auth_text 'claim_starter_oc' 'starter OC gameplay authority action'
  require_strong_auth_text 'starter_oc_required_gate' 'starter OC onboarding overlay scope'
  require_strong_auth_text 'overlay dismissal' 'starter OC overlay dismissal phase'
  if grep -En 'viewer-playthrough-action-claim-starter-oc' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- starter OC claim must use scoped visible text/XPath, not a test id"
  fi
  require_strong_auth_text 'entityCounts' 'empty-world entity count gate'
  require_strong_auth_text 'lastGameplayActionFeedback' 'first-agent gameplay authority ack wait'
  require_strong_auth_text 'claim first agent action' 'first-agent claim action phase label'
  if grep -En '__AW_TEST__\.(claim|sendGameplayAction)' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- first-agent claim must use the visible button, not the test API"
  fi
  require_strong_auth_text 'run_visible_action' 'visible action failure wrapper'
  require_strong_auth_text 'test-login action' 'test-login action phase label'
  if grep -En 'ab_cmd "\$SESSION" (click|fill).*\/dev\/null' "$runner"; then
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
  if grep -Fq -- 'wait --load networkidle' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- networkidle cannot be the sole long-lived Viewer readiness gate"
  fi

  require_strong_auth_text 'registerPlayerSessionForTest(null)' 'permitted hosted session-registration hook'
  require_strong_auth_text 'registerPlayerSessionForTest(${agent_id_json}, {forceRebind: true})' 'post-onboarding force-rebind hook'
  require_strong_auth_text 'post-onboarding auth binding rebind action' 'post-onboarding force-rebind diagnostics'
  require_strong_auth_text 'prompt surface page continuity' 'pre-prompt page-continuity guard'
  require_strong_auth_text 'a[href="#viewer-details-panel"]' 'visible Command panel navigation selector'
  require_strong_auth_text 'command panel navigation action' 'visible Command panel navigation phase'
  require_strong_auth_text 'section#viewer-details-panel[data-viewer-route-panel="command"]' 'exact Command route panel readiness'
  require_strong_auth_text '[data-prompt-visibility-toggle="1"]' 'exact prompt overrides toggle selector'
  require_strong_auth_text 'prompt overrides visibility action' 'visible prompt overrides toggle phase'
  require_strong_auth_text 'promptOverridesVisible === true' 'prompt overrides visible state readiness'
  require_strong_auth_text 'prompt overrides toggle visible and enabled' 'visible enabled prompt toggle readiness phase'
  require_strong_auth_text 'apply authoritative version convergence' 'post-Apply authoritative version convergence gate'
  require_strong_auth_text 'selectedPromptVersion' 'post-Apply selected version readback'
  require_strong_auth_text 'r.version) > beforeVersion' 'Apply response version advancement gate'
  require_strong_auth_text 'details.command-surface__advanced-details > summary' 'exact advanced prompt disclosure DOM selector'
  require_strong_auth_text 'pre-prompt tab loss' 'distinct about:blank tab-loss diagnostic'
  if grep -Fq -- 'wait --text "Advanced Prompt Settings"' "$runner"; then
    strong_auth_contract_failures="${strong_auth_contract_failures}\n- prompt readiness must use exact DOM state, not broad text wait"
  fi
  require_strong_auth_text 'hosted player session registration action' 'session-registration action phase label'
  require_strong_auth_text 'registered_unbound' 'unbound runtime registration wait'
  if grep -En 'ab_read_eval.*registerPlayerSessionForTest' "$runner"; then
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
run_hosted_local_mock_contract_checks
if "$runner" --contract-only --headed --hosted-local-mock --out-dir "$tmp_root/hosted-mock-without-tier" >"$tmp_root/hosted-mock-without-tier.log" 2>&1; then
  echo "Hosted local-mock route unexpectedly accepted without --test-tier-required" >&2
  exit 1
fi
grep -Fq -- "requires explicit --test-tier-required" "$tmp_root/hosted-mock-without-tier.log"
if "$runner" --contract-only --headed --hosted-local-mock --test-tier-required --full-gameplay --out-dir "$tmp_root/hosted-mock-full-gameplay" >"$tmp_root/hosted-mock-full-gameplay.log" 2>&1; then
  echo "Hosted local-mock route unexpectedly accepted with --full-gameplay" >&2
  exit 1
fi
grep -Fq -- "cannot be combined with --full-gameplay" "$tmp_root/hosted-mock-full-gameplay.log"

# The explicit seed-42 fixture must encode a complete Ed25519 key and the
# checked-in public half.  Keep this assertion secret-free: only lengths and
# the known public fixture are compared, never emitted.
python3 - "$runner" <<'PY'
import pathlib
import re
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
private = re.search(r'local seed42_private="([0-9a-f]+)"', source).group(1)
public = re.search(r'local seed42_public="([0-9a-f]+)"', source).group(1)
assert len(private) == 64, "seed-42 private fixture must contain 32 bytes"
assert len(bytes.fromhex(private)) == 32
assert len(public) == 64, "seed-42 public fixture must contain 32 bytes"
assert public == "197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61"
PY

# An explicit seed-42 Hosted route must not silently inherit a malformed
# signer from the caller environment.  The failure is intentionally generic:
# this test must never echo private key material into logs or artifacts.
bad_signer_out="$tmp_root/hosted-mock-bad-signer"
bad_signer_private="$(printf 'aa%.0s' {1..30})"
set +e
env \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture-public \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY="$bad_signer_private" \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture-approval \
  "$runner" \
  --contract-only \
  --headed \
  --hosted-local-mock \
  --test-tier-required \
  --test-signer-seed 42 \
  --out-dir "$bad_signer_out" >"$tmp_root/hosted-mock-bad-signer.log" 2>&1
bad_signer_rc=$?
set -e
test "$bad_signer_rc" -ne 0
grep -Fq -- "requires the deterministic seed-42 signer fixture" "$tmp_root/hosted-mock-bad-signer.log"
if grep -Fq -- "$bad_signer_private" "$tmp_root/hosted-mock-bad-signer.log"; then
  echo "bad signer fixture value leaked into runner diagnostics" >&2
  exit 1
fi

# A test signer is never an implicit production/default launcher input.  The
# no-seed contract remains caller/default mode and records no test signer.
production_contract_dir="$tmp_root/production-contract"
env \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=production-public \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=production-private \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=production-approval \
  "$runner" \
  --contract-only \
  --headed \
  --test-tier-required \
  --out-dir "$production_contract_dir" >/dev/null
python3 - "$production_contract_dir/runner-config.json" <<'PY'
import json
import pathlib
import sys

config = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert config["deploymentMode"] == "caller_or_default"
assert config["launchRoute"] == "legacy_prompt_control"
assert config["testSigner"] == {"privateKeyRecorded": False, "seed": None}
PY

if "$runner" --contract-only --headed --test-tier-required --test-signer-seed 42 --out-dir "$tmp_root/seed-without-hosted" >"$tmp_root/seed-without-hosted.log" 2>&1; then
  echo "explicit test signer unexpectedly accepted outside Hosted local-mock route" >&2
  exit 1
fi
grep -Fq -- "requires --hosted-local-mock" "$tmp_root/seed-without-hosted.log"

hosted_contract_dir="$tmp_root/hosted-local-mock-contract"
"$runner" \
  --contract-only \
  --headed \
  --hosted-local-mock \
  --test-tier-required \
  --test-signer-seed 42 \
  --out-dir "$hosted_contract_dir" >/dev/null
python3 - "$hosted_contract_dir/runner-config.json" "$hosted_contract_dir/artifact-manifest.json" <<'PY'
import json
import pathlib
import sys

config = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
assert config["browserMode"] == "headed"
assert config["launchRoute"] == "hosted_local_mock_seeded_agent"
assert config["deploymentMode"] == "hosted_public_join"
assert config["chain"] == {
    "autoPlay": False,
    "enabled": True,
    "standaloneTest": True,
    "storageProfile": "dev_local",
}
assert config["localAuthority"] == {
    "authorityArtifact": "runtime/local-test-provider-authority.json",
    "finalityBlockHash": "blake3:" + "0" * 64,
    "ownerBinding": "local-test-owner-0",
    "sessionMode": "hosted_public_join",
    "wasmArtifact": ".tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.wasm",
    "metadataArtifact": ".tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.metadata.json",
}
assert config["provider"] == {
    "backend": "provider_local_mock",
    "contract": "worldsim_provider_v1",
    "executionLane": "player_parity",
    "transport": "loopback_http",
    "url": "http://127.0.0.1:5841",
}
assert config["seededAgent"] is True
assert config["testSigner"] == {"privateKeyRecorded": False, "seed": 42}
assert config["testTierRequired"] is True
assert config["evidenceBoundary"] == {
    "providerCallsDuringVerification": False,
    "realInference": False,
    "worldConsequenceClaim": False,
}
assert manifest["launchRoute"] == config["launchRoute"]
assert manifest["testSignerSeed"] == 42
assert manifest["testTierRequired"] is True
assert manifest["providerCallsDuringVerification"] is False
assert manifest["externalProviderCallsDuringVerification"] is False
assert manifest["localMockCallsDuringVerification"] is True
assert "runner-config.json" in {item["path"] for item in manifest["artifacts"]}
PY
viewport_contract_dir="$tmp_root/viewport-contract"
"$runner" \
  --contract-only \
  --headed \
  --hosted-local-mock \
  --test-tier-required \
  --test-signer-seed 42 \
  --viewport-width 390 \
  --viewport-height 844 \
  --out-dir "$viewport_contract_dir" >/dev/null
python3 - "$viewport_contract_dir/runner-config.json" <<'PY'
import json
import pathlib
import sys

config = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert config["viewport"] == {"requestedHeight": 844, "requestedWidth": 390}
PY
set +e
"$runner" --contract-only --headed --viewport-width 390 --out-dir "$tmp_root/viewport-without-height" >"$tmp_root/viewport-without-height.log" 2>&1
viewport_missing_height_rc=$?
set -e
test "$viewport_missing_height_rc" -ne 0
grep -Fq -- "must be provided together" "$tmp_root/viewport-without-height.log"

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
if [[ "${1:-}" == "click" && "${2:-}" == 'a[href="#viewer-details-panel"]' && -n "${VIEWER_PROMPT_FIXTURE_COMMAND_NAV_COUNT:-}" ]]; then
  command_nav_count=0
  if [[ -f "$VIEWER_PROMPT_FIXTURE_COMMAND_NAV_COUNT" ]]; then
    command_nav_count=$(<"$VIEWER_PROMPT_FIXTURE_COMMAND_NAV_COUNT")
  fi
  printf '%s\n' "$((command_nav_count + 1))" >"$VIEWER_PROMPT_FIXTURE_COMMAND_NAV_COUNT"
  exit 0
fi
if [[ "${1:-}" == "click" && "${2:-}" == 'details.command-surface__advanced-details > summary' && -n "${VIEWER_PROMPT_FIXTURE_ADVANCED_DISCLOSURE_COUNT:-}" ]]; then
  advanced_disclosure_count=0
  if [[ -f "$VIEWER_PROMPT_FIXTURE_ADVANCED_DISCLOSURE_COUNT" ]]; then
    advanced_disclosure_count=$(<"$VIEWER_PROMPT_FIXTURE_ADVANCED_DISCLOSURE_COUNT")
  fi
  printf '%s\n' "$((advanced_disclosure_count + 1))" >"$VIEWER_PROMPT_FIXTURE_ADVANCED_DISCLOSURE_COUNT"
  exit 0
fi
if [[ "${1:-}" == "click" && "${2:-}" == '[data-prompt-visibility-toggle="1"]' && -n "${VIEWER_PROMPT_FIXTURE_PROMPT_TOGGLE_COUNT:-}" ]]; then
  if [[ -n "${VIEWER_PROMPT_FIXTURE_REQUIRE_ADVANCED_DISCLOSURE:-}" && ! -f "$VIEWER_PROMPT_FIXTURE_REQUIRE_ADVANCED_DISCLOSURE" ]]; then
    echo "fixture prompt visibility toggle clicked before Advanced disclosure" >&2
    exit 31
  fi
  prompt_toggle_count=0
  if [[ -f "$VIEWER_PROMPT_FIXTURE_PROMPT_TOGGLE_COUNT" ]]; then
    prompt_toggle_count=$(<"$VIEWER_PROMPT_FIXTURE_PROMPT_TOGGLE_COUNT")
  fi
  printf '%s\n' "$((prompt_toggle_count + 1))" >"$VIEWER_PROMPT_FIXTURE_PROMPT_TOGGLE_COUNT"
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
  elif [[ "$script" == *'window.innerWidth'* && "$script" == *'visualViewportWidth'* && "$script" != *'horizontalOverflowPx'* ]]; then
    printf '%s\n' '{"innerWidth":1280,"innerHeight":720,"outerWidth":1280,"outerHeight":720,"devicePixelRatio":1,"visualViewportWidth":1280,"visualViewportHeight":720}'
  elif [[ "$script" == *'horizontalOverflowPx'* ]]; then
    printf '%s\n' '{"activeElementIsRollback":true,"horizontalOverflowPx":0,"rollback":{"activeElement":"rollback","focusVisible":true,"height":44,"minHitTarget":true,"outlineStyle":"solid","outlineWidth":"2px","text":"Rollback Prompt","uncovered":true,"visible":true,"width":120},"targetRow":{"position":"sticky","visible":true},"viewport":{"height":720,"width":1280}}'
  elif [[ "$script" == *'prompt_surface_missing:'* && "$script" == *'details.command-surface__advanced-details > summary'* ]]; then
    if [[ "${VIEWER_PROMPT_FIXTURE_PRE_PROMPT_TAB_LOST:-0}" == "1" ]]; then
      printf '%s\n' '"tab_lost"'
    elif [[ "${VIEWER_PROMPT_FIXTURE_PRE_PROMPT_UI_MISSING:-0}" == "1" ]]; then
      printf '%s\n' '"prompt_surface_missing:http://127.0.0.1:9/"'
    elif [[ -n "${VIEWER_PROMPT_FIXTURE_REQUIRE_COMMAND_NAV:-}" && ! -f "$VIEWER_PROMPT_FIXTURE_REQUIRE_COMMAND_NAV" ]]; then
      printf '%s\n' '"prompt_surface_missing:http://127.0.0.1:9/#viewer-targets-panel"'
    else
      printf '%s\n' '"ready"'
    fi
  elif [[ "$script" == *'command-surface__advanced-details[open]'* && "$script" == *'data-prompt-visibility-toggle'* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_REQUIRE_ADVANCED_DISCLOSURE:-}" && ! -f "$VIEWER_PROMPT_FIXTURE_REQUIRE_ADVANCED_DISCLOSURE" ]]; then
      printf '%s\n' 'false'
    else
      printf '%s\n' 'true'
    fi
  elif [[ "$script" == *'promptOverridesVisible === true'* && "$script" == *'details.command-surface__advanced-details > summary'* ]]; then
    if [[ -n "${VIEWER_PROMPT_FIXTURE_REQUIRE_PROMPT_TOGGLE:-}" && ! -f "$VIEWER_PROMPT_FIXTURE_REQUIRE_PROMPT_TOGGLE" ]]; then
      printf '%s\n' 'false'
    else
      printf '%s\n' 'true'
    fi
  elif [[ "$script" == *'selectedPromptVersion'* && "$script" == *'r.version) > beforeVersion'* ]]; then
    convergence_read_count=0
    if [[ -n "${VIEWER_PROMPT_FIXTURE_APPLY_CONVERGENCE_COUNT:-}" && -f "$VIEWER_PROMPT_FIXTURE_APPLY_CONVERGENCE_COUNT" ]]; then
      convergence_read_count=$(<"$VIEWER_PROMPT_FIXTURE_APPLY_CONVERGENCE_COUNT")
    fi
    convergence_read_count=$((convergence_read_count + 1))
    if [[ -n "${VIEWER_PROMPT_FIXTURE_APPLY_CONVERGENCE_COUNT:-}" ]]; then
      printf '%s\n' "$convergence_read_count" >"$VIEWER_PROMPT_FIXTURE_APPLY_CONVERGENCE_COUNT"
    fi
    if [[ "${VIEWER_PROMPT_FIXTURE_LAG_APPLY_CONVERGENCE:-0}" == "1" && "$convergence_read_count" -lt 2 ]]; then
      printf '%s\n' 'false'
    else
      printf '%s\n' 'true'
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
fallback_backend_requirement="VIEWER_PROMPT_FIXTURE_REQUIRE_EXPLICIT_GL=1"
if [[ "$(uname -s)" == "Darwin" ]]; then
  fallback_backend_requirement="VIEWER_PROMPT_FIXTURE_REQUIRE_METAL=1"
fi
env "$fallback_backend_requirement" \
  VIEWER_PROMPT_FIXTURE_FAIL_DOM_WAIT=1 \
  VIEWER_PROMPT_FIXTURE_FALLBACK_MARKER="$fallback_marker" \
  OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
  OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
  PATH="$fake_bin:$PATH" "$runner" \
  --headed --test-login --url http://127.0.0.1:9 --out-dir "$fallback_out"
test -f "$fallback_out/agent-browser.log"
test -f "$fallback_marker"
grep -Fq 'domcontentloaded fallback' "$fallback_out/agent-browser.log"
test -f "$fallback_out/failure-domcontentloaded-session-info.json"
test -f "$fallback_out/failure-domcontentloaded-tabs.json"
python3 - "$fallback_out/browser-viewport.json" "$fallback_out/artifact-manifest.json" <<'PY'
import json
import pathlib
import sys

viewport = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
assert viewport["width"] == 1280
assert viewport["height"] == 720
assert viewport["innerWidth"] == 1280
assert viewport["innerHeight"] == 720
assert manifest["viewport"]["width"] == 1280
assert manifest["viewport"]["height"] == 720
layout = manifest["promptLayout"]
assert layout["horizontalOverflowPx"] == 0
assert layout["rollback"]["minHitTarget"] is True
assert layout["rollback"]["uncovered"] is True
assert layout["rollback"]["visible"] is True
assert layout["targetRow"]["position"] == "sticky"
PY

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
grep -Fq '[action:test-login action] command failed' "$action_failure_out/agent-browser.log"
test -f "$action_failure_out/failure-test-login_action-session-info.json"

claim_out="$tmp_root/claim-first-agent"
claim_count_file="$tmp_root/claim-first-agent-count"
panel_nav_count_file="$tmp_root/claim-first-agent-panel-nav-count"
session_registration_count_file="$tmp_root/claim-first-agent-session-registration-count"
claim_ack_marker="$tmp_root/claim-first-agent-ack-seen"
starter_oc_action_count_file="$tmp_root/claim-starter-oc-count"
starter_oc_ack_marker="$tmp_root/claim-starter-oc-ack-seen"
command_nav_count_file="$tmp_root/command-panel-nav-count"
prompt_toggle_count_file="$tmp_root/prompt-overrides-toggle-count"
advanced_disclosure_count_file="$tmp_root/advanced-disclosure-count"
apply_convergence_count_file="$tmp_root/apply-version-convergence-count"
VIEWER_PROMPT_FIXTURE_EMPTY_WORLD=1 \
  VIEWER_PROMPT_FIXTURE_STARTER_OC_ONBOARDING=1 \
  VIEWER_PROMPT_FIXTURE_CLAIM_ACTION_COUNT="$claim_count_file" \
  VIEWER_PROMPT_FIXTURE_PANEL_NAV_COUNT="$panel_nav_count_file" \
  VIEWER_PROMPT_FIXTURE_SESSION_REGISTRATION_COUNT="$session_registration_count_file" \
  VIEWER_PROMPT_FIXTURE_STARTER_OC_ACTION_COUNT="$starter_oc_action_count_file" \
  VIEWER_PROMPT_FIXTURE_STARTER_OC_ACK_MARKER="$starter_oc_ack_marker" \
  VIEWER_PROMPT_FIXTURE_CLAIM_ACK_MARKER="$claim_ack_marker" \
  VIEWER_PROMPT_FIXTURE_COMMAND_NAV_COUNT="$command_nav_count_file" \
  VIEWER_PROMPT_FIXTURE_REQUIRE_COMMAND_NAV="$command_nav_count_file" \
  VIEWER_PROMPT_FIXTURE_PROMPT_TOGGLE_COUNT="$prompt_toggle_count_file" \
  VIEWER_PROMPT_FIXTURE_REQUIRE_PROMPT_TOGGLE="$prompt_toggle_count_file" \
  VIEWER_PROMPT_FIXTURE_ADVANCED_DISCLOSURE_COUNT="$advanced_disclosure_count_file" \
  VIEWER_PROMPT_FIXTURE_REQUIRE_ADVANCED_DISCLOSURE="$advanced_disclosure_count_file" \
  VIEWER_PROMPT_FIXTURE_LAG_APPLY_CONVERGENCE=1 \
  VIEWER_PROMPT_FIXTURE_APPLY_CONVERGENCE_COUNT="$apply_convergence_count_file" \
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
test -f "$command_nav_count_file"
test "$(<"$command_nav_count_file")" = 1
test -f "$prompt_toggle_count_file"
test "$(<"$prompt_toggle_count_file")" = 1
test -f "$advanced_disclosure_count_file"
test "$(<"$advanced_disclosure_count_file")" = 1
test -f "$apply_convergence_count_file"
test "$(<"$apply_convergence_count_file")" -ge 2
grep -Fq '[action:claim first agent action]' "$claim_out/agent-browser.log"
grep -Fq '[action:command panel navigation action]' "$claim_out/agent-browser.log"
grep -Fq '[action:prompt overrides visibility action]' "$claim_out/agent-browser.log"

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
grep -Fq '[action:post-onboarding auth binding rebind action] completed' "$force_rebind_out/agent-browser.log"

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
grep -Fq '[action:post-onboarding auth binding rebind action] skipped; binding epoch already present or agent not bound' "$force_rebind_skip_out/agent-browser.log"

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
grep -Fq '[prompt surface page continuity] tab lost to about:blank' "$tab_loss_out/agent-browser.log"
test -f "$tab_loss_out/failure-pre-prompt_tab_loss-tabs.json"
if grep -Fq '[action:advanced prompt disclosure action]' "$tab_loss_out/agent-browser.log"; then
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
assert paths == ["contract.json", "manifest-input.json", "runner-config.json"]
for entry in manifest["artifacts"]:
    assert set(entry) == {"bytes", "path", "sha256"}
    assert len(entry["sha256"]) == 64
PY

if "$runner" --headless --contract-only --out-dir "$tmp_root/headless" >"$tmp_root/headless.log" 2>&1; then
  echo "headless contract unexpectedly passed" >&2
  exit 1
fi
grep -Fq -- "--headed is required" "$tmp_root/headless.log"

if grep -En 'sendPromptControl|__AW_TEST__\.sendPromptControl' "$runner"; then
  echo "runner must not replace visible prompt actions with sendPromptControl" >&2
  exit 1
fi
grep -Fq 'data-pixel-world-agent-marker' "$runner"
grep -Fq 'fill "#prompt-short"' "$runner"
grep -Fq 'button[data-prompt-action="preview"]' "$runner"
grep -Fq 'button[data-prompt-action="apply"]' "$runner"
grep -Fq 'button[data-prompt-action="rollback"]' "$runner"

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
[[ -n "${OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY:-}" ]]
[[ -n "${OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY:-}" ]]
[[ -n "${OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE:-}" ]]
[[ "${OASIS7_HOSTED_TEST_LOGIN_ENABLED:-}" == "1" ]]
output_dir=""
chain_disable_seen=0
chain_enable_seen=0
args_file="${VIEWER_PROMPT_FIXTURE_ARGS_FILE:-}"
if [[ -n "$args_file" ]]; then
  : >"$args_file"
fi
while (($# > 0)); do
  if [[ -n "$args_file" ]]; then
    printf '%s\n' "$1" >>"$args_file"
  fi
  if [[ "${1:-}" == "--output-dir" ]]; then
    output_dir="${2:?missing output dir}"
    shift 2
  elif [[ "${1:-}" == "--chain-disable" ]]; then
    chain_disable_seen=1
    shift
  elif [[ "${1:-}" == "--chain-enable" ]]; then
    chain_enable_seen=1
    shift
  else
    shift
  fi
done
case "${VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE:-default}" in
  default) [[ "$chain_disable_seen" == "1" ]] ;;
  hosted) [[ "$chain_enable_seen" == "1" ]] ;;
  full) [[ "$chain_enable_seen" == "1" ]] ;;
  *) exit 1 ;;
esac
mkdir -p "$output_dir"
printf '%s\n' '{"state":"before-close"}' >"$output_dir/hosted-player-sessions.json"
if [[ "${VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE:-default}" == "full" ]]; then
  cat >"$output_dir/session.meta" <<'META'
STACK_READY=1
LOCAL_TEST_PROVIDER_SETUP_ENABLED=1
CHAIN_ENABLED=1
DEPLOYMENT_MODE=trusted_local_only
META
  cat >"$output_dir/local-test-provider-authority.json" <<'JSON'
{"agent_id":"starter-agent-0","grant":{"grant_id":"fixture-grant"},"invocation_context":{"grant_id":"fixture-grant","subject":{"agent_id":"starter-agent-0"}}}
JSON
fi
printf '%s\n' '- URL: http://127.0.0.1:9'
printf '%s\n' '- URL: http://127.0.0.1:9/?render_mode=viewer&ws=ws%3A%2F%2F127.0.0.1%3A11&hosted_access=fixture-authority' \
  >"$output_dir/oasis7_viewer_live.log"
sleep 5
EOF
  mkdir -p "$sandbox/.tmp/wasm-build-suite/local-test-provider" "$sandbox/fake-target"
  : >"$sandbox/.tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.wasm"
  : >"$sandbox/.tmp/wasm-build-suite/local-test-provider/module.runtime.local-test-provider.metadata.json"
  cat >"$sandbox/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
args=("$@")
if [[ "${args[0]:-}" == "-C" ]]; then
  args=("${args[@]:2}")
fi
case "${args[*]}" in
  "rev-parse --verify HEAD^{commit}") printf '%s\n' 'fixture-head-7181473f' ;;
  "rev-parse --verify fixture-base^{commit}") printf '%s\n' 'fixture-base-65e2cc9' ;;
  "merge-base --is-ancestor fixture-base-65e2cc9 fixture-head-7181473f") exit 0 ;;
  "status --porcelain --untracked-files=all")
    if [[ "${VIEWER_PROMPT_FIXTURE_GIT_DIRTY:-0}" == "1" ]]; then
      printf '%s\n' ' M source.rs'
    fi
    ;;
  *) echo "unsupported fixture git call: ${args[*]}" >&2; exit 97 ;;
esac
EOF
  cat >"$sandbox/bin/rustc" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-vV" ]]; then
  printf '%s\n' 'release: fixture-rustc' 'host: fixture-host'
fi
EOF
  cat >"$sandbox/bin/cargo" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' 'cargo fixture-cargo'
EOF
  cat >"$sandbox/bin/rustup" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' 'fixture-toolchain'
EOF
  cat >"$sandbox/scripts/cargo-dev-lib.sh" <<'EOF'
#!/usr/bin/env bash
oasis7_cargo_dev() {
  local expect_bin=""
  while (($# > 0)); do
    if [[ "${1:-}" == "--bin" ]]; then
      expect_bin="${2:?missing binary}"
      : >"${OASIS7_TEST_TIER_FAKE_TARGET:?missing fake target}/$expect_bin"
      chmod +x "${OASIS7_TEST_TIER_FAKE_TARGET}/$expect_bin"
      shift 2
    else
      shift
    fi
  done
}
oasis7_cargo_dev_debug_bin_dir() {
  printf '%s\n' "${OASIS7_TEST_TIER_FAKE_TARGET:?missing fake target}"
}
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
  elif [[ "$script" == *'window.innerWidth'* && "$script" == *'visualViewportWidth'* && "$script" != *'horizontalOverflowPx'* ]]; then
    printf '%s\n' '{"innerWidth":1280,"innerHeight":720,"outerWidth":1280,"outerHeight":720,"devicePixelRatio":1,"visualViewportWidth":1280,"visualViewportHeight":720}'
  elif [[ "$script" == *'horizontalOverflowPx'* ]]; then
    printf '%s\n' '{"activeElementIsRollback":true,"horizontalOverflowPx":0,"rollback":{"activeElement":"rollback","focusVisible":true,"height":44,"minHitTarget":true,"outlineStyle":"solid","outlineWidth":"2px","text":"Rollback Prompt","uncovered":true,"visible":true,"width":120},"targetRow":{"position":"sticky","visible":true},"viewport":{"height":720,"width":1280}}'
  elif [[ "$script" == *'prompt_surface_missing:'* && "$script" == *'details.command-surface__advanced-details > summary'* ]]; then
    printf '%s\n' '"ready"'
  else
    printf '%s\n' 'true'
  fi
elif [[ " $* " == *" console "* ]]; then
  if [[ "${VIEWER_PROMPT_FIXTURE_CONSOLE_FAILURE:-0}" == "1" ]]; then
    echo "fixture console capture failed" >&2
    exit 17
  fi
  printf '%s\n' '[]'
elif [[ " $* " == *" errors "* ]]; then
  printf '%s\n' '[]'
elif [[ " $* " == *" close "* ]]; then
  if [[ "${VIEWER_PROMPT_FIXTURE_MUTATE_ON_CLOSE:-0}" == "1" && -n "${AB_SESSION_ARTIFACT_DIR:-}" ]]; then
    printf '%s\n' '{"state":"after-close"}' >"$AB_SESSION_ARTIFACT_DIR/runtime/hosted-player-sessions.json"
  fi
fi
exit 0
EOF
  chmod +x "$sandbox/scripts/run-launcher-stack.sh" "$sandbox/bin/agent-browser" "$sandbox/bin/git" "$sandbox/bin/rustc" "$sandbox/bin/cargo" "$sandbox/bin/rustup"
  set +e
  env "$fallback_backend_requirement" \
    VIEWER_PROMPT_FIXTURE_REQUIRE_HOSTED_ACCESS=1 \
    VIEWER_PROMPT_FIXTURE_REQUIRE_NO_PROXY=1 \
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

  hosted_args_file="$tmp_root/hosted-stack-args"
  env "$fallback_backend_requirement" \
    VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE=hosted \
    VIEWER_PROMPT_FIXTURE_MUTATE_ON_CLOSE=1 \
    VIEWER_PROMPT_FIXTURE_ARGS_FILE="$hosted_args_file" \
    OASIS7_TEST_TIER_FAKE_TARGET="$sandbox/fake-target" \
    PATH="$sandbox/bin:$PATH" /bin/bash "$sandbox/scripts/viewer-prompt-control-regression.sh" \
    --headed --hosted-local-mock --test-tier-required --test-login \
    --out-dir "$tmp_root/hosted-stack-args-run" \
    --source-base fixture-base \
    --caller-sentinel hosted-value
grep -Fqx -- '--caller-sentinel' "$hosted_args_file"
grep -Fqx -- 'hosted-value' "$hosted_args_file"
python3 - "$tmp_root/hosted-stack-args-run/binary-provenance.json" "$tmp_root/hosted-stack-args-run/runner-config.json" "$tmp_root/hosted-stack-args-run/artifact-manifest.json" <<'PY'
import hashlib
import json
import pathlib
import sys

provenance = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
config = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
manifest = json.loads(pathlib.Path(sys.argv[3]).read_text(encoding="utf-8"))
assert provenance["schema"] == "oasis7.viewer.binary-provenance/v1"
assert provenance["source"]["head"] == "fixture-head-7181473f"
assert provenance["source"]["base"] == "fixture-base-65e2cc9"
assert provenance["source"]["baseRef"] == "fixture-base"
assert provenance["source"]["treeClean"] is True
assert provenance["build"]["profile"] == "debug"
assert provenance["build"]["features"] == ["test_tier_required"]
assert provenance["build"]["command"][:4] == ["oasis7_cargo_dev", "build", "-p", "oasis7"]
assert provenance["build"]["toolchain"]["rustcVersion"] == "fixture-rustc"
assert provenance["build"]["toolchain"]["cargoVersion"] == "fixture-cargo"
assert provenance["status"] == "verified"
assert set(provenance["binaries"]) == {
    "oasis7_chain_runtime",
    "oasis7_game_launcher",
    "oasis7_llm_provider_probe",
    "oasis7_viewer_live",
}
for entry in provenance["binaries"].values():
    assert len(entry["sha256"]) == 64
    assert entry["path"]
assert config["provenance"]["path"] == "binary-provenance.json"
assert config["source"]["head"] == "fixture-head-7181473f"
assert manifest["binaryProvenance"]["status"] == "verified"
assert "binary-provenance.json" in {item["path"] for item in manifest["artifacts"]}
assert manifest["browserDiagnostics"] == {
    "console": "browser-console.log",
    "errors": "browser-errors.log",
}
assert pathlib.Path(sys.argv[3]).parent.joinpath("browser-console.log").read_text(encoding="utf-8").strip() == "[]"
assert pathlib.Path(sys.argv[3]).parent.joinpath("browser-errors.log").read_text(encoding="utf-8").strip() == "[]"
for item in manifest["artifacts"]:
    path = pathlib.Path(sys.argv[3]).parent / item["path"]
    assert path.is_file(), item["path"]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == item["sha256"], item["path"]
PY

diagnostic_failure_out="$tmp_root/browser-diagnostics-failure"
set +e
env "$fallback_backend_requirement" \
  VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE=hosted \
  VIEWER_PROMPT_FIXTURE_CONSOLE_FAILURE=1 \
  OASIS7_TEST_TIER_FAKE_TARGET="$sandbox/fake-target" \
  PATH="$sandbox/bin:$PATH" /bin/bash "$sandbox/scripts/viewer-prompt-control-regression.sh" \
  --headed --hosted-local-mock --test-tier-required --test-login \
  --source-base fixture-base --out-dir "$diagnostic_failure_out" >"$tmp_root/browser-diagnostics-failure.log" 2>&1
diagnostic_rc=$?
set -e
test "$diagnostic_rc" -ne 0
grep -Fq -- "requires browser console/errors capture" "$tmp_root/browser-diagnostics-failure.log"
test ! -e "$diagnostic_failure_out/artifact-manifest.json"

stale_out="$tmp_root/stale-provenance-output"
mkdir -p "$stale_out"
printf '%s\n' '{"schema":"oasis7.viewer.binary-provenance/v1","status":"verified"}' >"$stale_out/binary-provenance.json"
set +e
env "$fallback_backend_requirement" \
  VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE=hosted \
  OASIS7_TEST_TIER_FAKE_TARGET="$sandbox/fake-target" \
  PATH="$sandbox/bin:$PATH" /bin/bash "$sandbox/scripts/viewer-prompt-control-regression.sh" \
  --headed --hosted-local-mock --test-tier-required --test-login \
  --source-base fixture-base --out-dir "$stale_out" >"$tmp_root/stale-provenance.log" 2>&1
stale_rc=$?
set -e
test "$stale_rc" -ne 0
grep -Fq -- "refusing to reuse existing provenance output" "$tmp_root/stale-provenance.log"

dirty_out="$tmp_root/dirty-source-output"
set +e
env "$fallback_backend_requirement" \
  VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE=hosted \
  VIEWER_PROMPT_FIXTURE_GIT_DIRTY=1 \
  OASIS7_TEST_TIER_FAKE_TARGET="$sandbox/fake-target" \
  PATH="$sandbox/bin:$PATH" /bin/bash "$sandbox/scripts/viewer-prompt-control-regression.sh" \
  --headed --hosted-local-mock --test-tier-required --test-login \
  --source-base fixture-base --out-dir "$dirty_out" >"$tmp_root/dirty-source.log" 2>&1
dirty_rc=$?
set -e
test "$dirty_rc" -ne 0
grep -Fq -- "source tree is dirty; refusing exact-head binary provenance" "$tmp_root/dirty-source.log"

  full_args_file="$tmp_root/full-gameplay-stack-args"
  env "$fallback_backend_requirement" \
    VIEWER_PROMPT_FIXTURE_EXPECT_ROUTE=full \
    VIEWER_PROMPT_FIXTURE_ARGS_FILE="$full_args_file" \
    OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=fixture \
    OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=fixture \
    OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=fixture \
    PATH="$sandbox/bin:$PATH" /bin/bash "$sandbox/scripts/viewer-prompt-control-regression.sh" \
    --headed --full-gameplay --test-login \
    --out-dir "$tmp_root/full-gameplay-stack-args-run" \
    --caller-sentinel full-value
grep -Fqx -- '--caller-sentinel' "$full_args_file"
grep -Fqx -- 'full-value' "$full_args_file"
fi

grep -Fq 'if ((${#STACK_ARGS[@]} > 0)); then' "$runner"

echo "viewer-prompt-control runner contract: passed"
