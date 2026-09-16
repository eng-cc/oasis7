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

  require_strong_auth_text 'registerPlayerSessionForTest' 'permitted server binding hook'
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
  exit 0
fi
if [[ "${1:-}" == "eval" && "${2:-}" == "--stdin" ]]; then
  script=$(cat)
  if [[ "$script" == 'window.__AW_TEST__.getState()' ]]; then
    printf '%s\n' '{"authReady":true,"authRegistrationStatus":"registered","authRuntimeStatus":"registered","authBoundAgentId":"agent-1","authSessionEpoch":1,"authBindingEpoch":1,"viewerProtocol":{"negotiated":true,"capabilities":["prompt_control_result_v1"],"authorityEpoch":"fixture-authority"},"selectedId":"agent-1","selectedPromptVersion":0,"lastPromptFeedback":null,"strongAuthLastGrantActionId":null,"strongAuthLastGrantError":null}'
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
printf '%s\n' '- URL: http://127.0.0.1:9'
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
elif [[ "${1:-}" == "eval" && "${2:-}" == "--stdin" ]]; then
  script=$(cat)
  if [[ "$script" == 'window.__AW_TEST__.getState()' ]]; then
    printf '%s\n' '{"authReady":true,"authRegistrationStatus":"registered","authRuntimeStatus":"registered","authBoundAgentId":"agent-1","authSessionEpoch":1,"authBindingEpoch":1,"viewerProtocol":{"negotiated":true,"capabilities":["prompt_control_result_v1"],"authorityEpoch":"fixture-authority"},"selectedId":"agent-1","selectedPromptVersion":0,"lastPromptFeedback":null,"strongAuthLastGrantActionId":null,"strongAuthLastGrantError":null}'
  else
    printf '%s\n' 'true'
  fi
fi
exit 0
EOF
  chmod +x "$sandbox/scripts/run-launcher-stack.sh" "$sandbox/bin/agent-browser"
  set +e
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
