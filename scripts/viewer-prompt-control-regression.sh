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

visible_action_contract() {
  cat <<'EOF'
{"selection":"[data-pixel-world-agent-marker=\"true\"][data-agent-id]","promptDisclosure":"Advanced Prompt Settings","shortTermGoal":"#prompt-short","preview":"button[data-prompt-action=\"preview\"]","apply":"button[data-prompt-action=\"apply\"]","rollback":"button[data-prompt-action=\"rollback\"]"}
EOF
}

write_manifest() {
  local root="$1"
  local tier="$2"
  local eligible="$3"
  local provider_calls="$4"
  python3 - "$root" "$CASE_ID" "$tier" "$eligible" "$provider_calls" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
case_id, tier = sys.argv[2], sys.argv[3]
eligible = sys.argv[4].lower() == "true"
provider_calls = sys.argv[5].lower() == "true"
visible = {
    "selection": r'[data-pixel-world-agent-marker=\"true\"][data-agent-id]',
    "promptDisclosure": "Advanced Prompt Settings",
    "shortTermGoal": "#prompt-short",
    "preview": r'button[data-prompt-action=\"preview\"]',
    "apply": r'button[data-prompt-action=\"apply\"]',
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
    "selection": r'[data-pixel-world-agent-marker=\"true\"][data-agent-id]',
    "promptDisclosure": "Advanced Prompt Settings",
    "shortTermGoal": "#prompt-short",
    "preview": r'button[data-prompt-action=\"preview\"]',
    "apply": r'button[data-prompt-action=\"apply\"]',
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

GAME_URL="$(python3 - "$GAME_URL" <<'PY'
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
import sys
parts = urlsplit(sys.argv[1])
query = dict(parse_qsl(parts.query, keep_blank_values=True))
query.update({"render_mode": "viewer", "test_api": "1"})
print(urlunsplit((parts.scheme, parts.netloc, parts.path, urlencode(query), parts.fragment)))
PY
)"

ab_open "$SESSION" 1 "$GAME_URL"
ab_read_retry "$SESSION" wait --load networkidle >/dev/null
AGENT_BROWSER_DEFAULT_TIMEOUT="$ACTION_TIMEOUT_MS" \
  ab_read_retry "$SESSION" wait --text "Advanced Prompt Settings" >/dev/null
ab_read_eval "$SESSION" 'window.__AW_TEST__.getState()' >"$OUT_DIR/state-before.json"

# All control changes below are visible browser actions. The test API is used
# only for state readback and artifact capture, never to select, type, submit,
# or replace the prompt-control UI interaction.
ab_cmd "$SESSION" click '[data-pixel-world-agent-marker="true"][data-agent-id]' >/dev/null
ab_cmd "$SESSION" click '.pixel-world-focus-entry__button' >/dev/null 2>&1 || true
ab_cmd "$SESSION" click '.pixel-world-focus-control--primary' >/dev/null 2>&1 || true
ab_cmd "$SESSION" click 'details.command-surface__advanced-details > summary' >/dev/null 2>&1 || true
ab_cmd "$SESSION" fill "#prompt-short" "$PROMPT_GOAL" >/dev/null
ab_cmd "$SESSION" click 'button[data-prompt-action="preview"]' >/dev/null
ab_read_eval "$SESSION" 'window.__AW_TEST__.getState()' >"$OUT_DIR/state-after-preview.json"
ab_cmd "$SESSION" click 'button[data-prompt-action="apply"]' >/dev/null
ab_read_eval "$SESSION" 'window.__AW_TEST__.getState()' >"$OUT_DIR/state-after-apply.json"
ab_cmd "$SESSION" click 'button[data-prompt-action="rollback"]' >/dev/null
ab_read_eval "$SESSION" 'window.__AW_TEST__.getState()' >"$OUT_DIR/state-after-rollback.json"
ab_screenshot "$SESSION" "$OUT_DIR/prompt-control.png" >/dev/null
ab_cmd "$SESSION" snapshot >/dev/null 2>&1 || true

python3 - "$OUT_DIR" "$CASE_ID" "$AGENT_ID" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
summary = {
    "acceptanceEligible": False,
    "agentId": sys.argv[3],
    "blockedCases": {
        "AC-PROMPT-007": "requires a real second authorized actor revocation or transfer",
        "AC-PROMPT-009": "requires ordinary and race-path convergence with exactly one authority result",
        "AC-PROMPT-011": "requires reconnect and other-entry observation before persistence or sync claims",
    },
    "caseId": sys.argv[2],
    "evidenceTier": "real_browser_baseline_partial",
    "note": "Baseline visible headed flow only; this artifact is not W3 acceptance evidence.",
}
(root / "run-summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
write_manifest "$OUT_DIR" "real_browser_baseline_partial" false true
echo "headed baseline complete; W3 acceptance remains blocked (manifest: $OUT_DIR/artifact-manifest.json)"
exit 3
