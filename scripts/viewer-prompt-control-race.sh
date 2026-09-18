#!/usr/bin/env bash
set -euo pipefail
umask 077

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASE_ID="PWT-004"
OUT_DIR="output/playwright/prompt-control/${CASE_ID}-race"
CONTRACT_ONLY=0
LIVE=0
HEADED=0
GAME_URL=""
AUTH_BOUNDARY_PROOF=""
AGENT_ID="starter-agent-0"
RUN_ID="viewer-prompt-control-race-$(date -u +%Y%m%dT%H%M%SZ)-$$"

# Both real actors are tabs in one owned browser session.  Stable tab IDs are
# resolved after creation and used explicitly before every actor operation.
SESSION=""
TAB_A=""
TAB_B=""
ARTIFACT_DIR_A=""
ARTIFACT_DIR_B=""
REQUEST_ID_A=""
REQUEST_ID_B=""
APPLY_HANDLE_A=""
APPLY_HANDLE_B=""
CDP_URL=""
DISPATCH_RELEASE=""
DISPATCH_READY_A=""
DISPATCH_READY_B=""
DISPATCH_RECEIPT_A=""
DISPATCH_RECEIPT_B=""
DISPATCH_PID_A=""
DISPATCH_PID_B=""
DISPATCH_RELEASE_AT_NS=""

usage() {
  cat <<'EOF'
Usage: viewer-prompt-control-race.sh --contract-only

The live two-actor lane is fail-closed until runtime-owned auth-boundary
proof is supplied. Contract-only mode performs no browser/provider calls.

Options:
  --contract-only
  --live --headed --url URL --auth-boundary-proof PATH [--agent-id ID]
  --out-dir DIR
EOF
}

while (($# > 0)); do
  case "$1" in
    --contract-only) CONTRACT_ONLY=1 ;;
    --live) LIVE=1 ;;
    --headed) HEADED=1 ;;
    --url) shift; GAME_URL="${1:?missing value for --url}" ;;
    --auth-boundary-proof) shift; AUTH_BOUNDARY_PROOF="${1:?missing value for --auth-boundary-proof}" ;;
    --out-dir) shift; OUT_DIR="${1:?missing value for --out-dir}" ;;
    --agent-id) shift; AGENT_ID="${1:?missing value for --agent-id}" ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unsupported option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

if (( CONTRACT_ONLY == 1 && LIVE == 1 )); then
  echo "error: --contract-only and --live are mutually exclusive" >&2
  exit 2
fi
if (( CONTRACT_ONLY == 0 && LIVE == 0 )); then
  echo "error: choose --contract-only, or --live with an auth-boundary proof" >&2
  exit 2
fi
if (( LIVE == 1 && HEADED == 0 )); then
  echo "error: --headed is required for the live browser lane" >&2
  exit 2
fi

ARTIFACT_DIR_A="$OUT_DIR/actor-a"
ARTIFACT_DIR_B="$OUT_DIR/actor-b"

write_safe_state() {
  local raw_json="$1"
  local out_path="$2"
  python3 - "$raw_json" "$out_path" <<'PY'
import json
import pathlib
import sys

raw, out_path = sys.argv[1], pathlib.Path(sys.argv[2])
sensitive_keys = {
    "authPlayerId", "authPublicKey", "authRevokedBy",
    "authRecoveryErrorMessage", "privateKey", "publicKey",
    "releaseToken", "registrationGrant", "approvalCode", "submittedDraft",
    "systemPrompt", "shortTermGoal", "longTermGoal",
    "system_prompt_override", "short_term_goal_override", "long_term_goal_override",
}

def redact(value):
    if isinstance(value, dict):
        return {key: redact(item) for key, item in value.items() if key not in sensitive_keys}
    if isinstance(value, list):
        return [redact(item) for item in value]
    return value

try:
    data = json.loads(raw)
except Exception:
    data = {"stateReadback": "unavailable"}
out_path.parent.mkdir(parents=True, exist_ok=True)
try:
    out_path.parent.chmod(0o700)
except OSError:
    pass
out_path.write_text(json.dumps(redact(data), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}

write_actor_receipt() {
  local actor_label="$1"
  local artifact_dir="$2"
  local session="$3"
  local request_id="$4"
  local outcome="${5:-not-issued}"
  local reason_code="${6:-}"
  local expected_version="${7:-}"
  local response_json="${8:-{}}"
  local baseline_player="${9:-}"
  python3 - "$artifact_dir/actor-receipt.json" "$actor_label" "$session" "$request_id" "$outcome" "$reason_code" "$expected_version" "$response_json" "$baseline_player" <<'PY'
import hashlib
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.parent.chmod(0o700)
try:
    response = json.loads(sys.argv[8])
except (TypeError, ValueError):
    response = {}
if not isinstance(response, dict):
    response = {}
baseline_player = sys.argv[9]
response_player = response.get("player_id")
def optional_int(value):
    return value if isinstance(value, int) and not isinstance(value, bool) else None

receipt = {
    "actor": sys.argv[2],
    "expected_version": int(sys.argv[7]) if sys.argv[7].isdigit() else None,
    "outcome": sys.argv[5],
    "request_id": response.get("request_id") or sys.argv[4] or None,
    "reason_code": response.get("reason_code") or sys.argv[6] or None,
    "response": {
        "status": response.get("status"),
        "authority_epoch": response.get("authority_epoch"),
        "agent_id": response.get("agent_id"),
        "player_id_present": isinstance(response_player, str) and bool(response_player),
        "player_id_matches_baseline": bool(baseline_player) and response_player == baseline_player,
        "session_epoch": optional_int(response.get("session_epoch")),
        "binding_epoch": optional_int(response.get("binding_epoch")),
        "expected_version": optional_int(response.get("expected_version")),
        "version": optional_int(response.get("version")),
        "current_version": optional_int(response.get("current_version")),
        "mutation_count": optional_int(response.get("mutation_count")),
        "applied_scope": response.get("applied_scope"),
        "persistence_scope": response.get("persistence_scope"),
        "sync_scope": response.get("sync_scope"),
        "value_visibility": response.get("value_visibility"),
        "operation_digest": response.get("operation_digest"),
        "idempotent_replay": response.get("idempotent_replay", False),
    },
    "session": "owned-session-redacted" if sys.argv[3] else "not-started",
    "promptSubmission": "apply" if sys.argv[5] in {"applied", "stale"} else "not-issued",
}
path.write_text(json.dumps({
    **receipt,
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

ensure_secure_artifact_root() {
  mkdir -p "$OUT_DIR" "$ARTIFACT_DIR_A" "$ARTIFACT_DIR_B"
  chmod 700 "$OUT_DIR" "$ARTIFACT_DIR_A" "$ARTIFACT_DIR_B"
}

write_manifest() {
  python3 - "$OUT_DIR" "$CASE_ID" "$AUTH_BOUNDARY_PROOF" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
case_id = sys.argv[2]
proof = sys.argv[3]
results_path = root / "race-results.json"
expected_outcome_counts = {"applied": 1, "stale": 1}
try:
    results = json.loads(results_path.read_text(encoding="utf-8"))
except (OSError, ValueError):
    results = {}
try:
    cleanup = json.loads((root / "cleanup.json").read_text(encoding="utf-8"))
except (OSError, ValueError):
    cleanup = {"status": "not-recorded", "acceptanceInvalidated": True}
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
eligible = results.get("acceptanceEligible") is True \
    and cleanup.get("status") == "passed" \
    and cleanup.get("acceptanceInvalidated") is not True
manifest = {
    "acceptanceEligible": eligible,
    "actors": results.get("actors", [
        {"label": "A", "artifactDir": "actor-a", "request_id": "not-issued"},
        {"label": "B", "artifactDir": "actor-b", "request_id": "not-issued"},
    ]),
    "artifacts": artifacts,
    "authBoundary": {
        "eligibility": "authorized" if eligible and proof else "not-asserted",
        "proofSupplied": bool(proof),
    },
    "browserMode": "headed",
    "caseId": case_id,
    "evidenceTier": results.get("evidenceTier", "contract_only"),
    "expectedOutcomeCounts": expected_outcome_counts,
    "outcomeCounts": results.get("outcomeCounts", {"applied": 0, "stale": 0}),
    "baseline": results.get("baseline", {}),
    "cleanup": cleanup,
    "providerCallsDuringVerification": False,
    "runtimeActionsDuringVerification": results.get("runtimeActionsDuringVerification", []),
    "schema": "oasis7.viewer.prompt-control-race-artifact-manifest/v1",
    "visibleActionContract": {
        "promptSubmission": "apply" if eligible else "not-issued",
    },
}
root.chmod(0o700)
(root / "artifact-manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY
}

write_contract_artifacts() {
  ensure_secure_artifact_root
  write_actor_receipt "A" "$ARTIFACT_DIR_A" "" "$REQUEST_ID_A"
  write_actor_receipt "B" "$ARTIFACT_DIR_B" "" "$REQUEST_ID_B"
  python3 - "$OUT_DIR/contract.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.write_text(json.dumps({
    "acceptanceEligible": False,
    "authBoundary": "not_asserted",
    "caseId": "PWT-004",
    "evidenceTier": "contract_only",
    "providerCallsDuringVerification": False,
    "runtimeActionsDuringVerification": [],
    "sessionSurface": {
        "ownedSession": "SESSION",
        "actorA": {"tab": "TAB_A", "artifactDir": "actor-a"},
        "actorB": {"tab": "TAB_B", "artifactDir": "actor-b"},
    },
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
  echo "race contract-only artifacts written to $OUT_DIR"
}

require_auth_boundary() {
  if [[ -z "$AUTH_BOUNDARY_PROOF" || ! -f "$AUTH_BOUNDARY_PROOF" ]]; then
    echo "error: live dual-actor lane is blocked; runtime auth-boundary proof is required" >&2
    return 2
  fi
  python3 - "$AUTH_BOUNDARY_PROOF" <<'PY'
import json
import pathlib
import sys

try:
    data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
except (OSError, ValueError):
    raise SystemExit("invalid auth-boundary proof")
if data.get("schema") != "oasis7.viewer.two-actor-auth-boundary/v1":
    raise SystemExit("unsupported auth-boundary proof schema")
if data.get("status") != "authorized":
    raise SystemExit("auth-boundary proof is not authorized")
PY
}

write_cleanup_evidence() {
  local cleanup_status="$1"
  local session_cleanup_status="$2"
  local acceptance_invalidated="$3"
  python3 - "$OUT_DIR/cleanup.json" "$cleanup_status" "$session_cleanup_status" "$acceptance_invalidated" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.parent.chmod(0o700)
path.write_text(json.dumps({
    "acceptanceInvalidated": sys.argv[4].lower() == "true",
    "sessionCleanup": sys.argv[3],
    "status": sys.argv[2],
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

invalidate_acceptance_for_cleanup() {
  local cleanup_status="$1"
  if [[ ! -f "$OUT_DIR/race-results.json" ]]; then
    return 0
  fi
  python3 - "$OUT_DIR/race-results.json" "$cleanup_status" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
try:
    data = json.loads(path.read_text(encoding="utf-8"))
except (OSError, ValueError):
    data = {}
data["acceptanceEligible"] = False
data["cleanupFailure"] = {
    "status": sys.argv[2],
    "acceptanceInvalidated": True,
}
path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  local session_cleanup_status="not-started"
  local cleanup_status="passed"
  if [[ -n "$APPLY_HANDLE_A" && -n "$CDP_URL" ]]; then
    if ! ab_run "$APPLY_HANDLE_A" --cdp "$CDP_URL" close >/dev/null 2>&1; then
      cleanup_status="failed"
      session_cleanup_status="failed"
    fi
  fi
  if [[ -n "$APPLY_HANDLE_B" && -n "$CDP_URL" ]]; then
    if ! ab_run "$APPLY_HANDLE_B" --cdp "$CDP_URL" close >/dev/null 2>&1; then
      cleanup_status="failed"
      session_cleanup_status="failed"
    fi
  fi
  if [[ -n "$SESSION" ]]; then
    if ab_session_cleanup "$SESSION" "$OUT_DIR"; then
      session_cleanup_status="passed"
    else
      session_cleanup_status="failed"
      cleanup_status="failed"
    fi
  fi
  if [[ "$cleanup_status" == "failed" ]]; then
    invalidate_acceptance_for_cleanup "$session_cleanup_status"
  fi
  write_cleanup_evidence "$cleanup_status" "$session_cleanup_status" \
    "$([[ "$cleanup_status" == "failed" ]] && echo true || echo false)"
  if ! write_manifest; then
    cleanup_status="failed"
    invalidate_acceptance_for_cleanup "manifest-write-failed"
    write_cleanup_evidence "$cleanup_status" "manifest-write-failed" true || true
    write_manifest || true
  fi
  if [[ "$cleanup_status" == "failed" && "$exit_code" -eq 0 ]]; then
    exit_code=1
  fi
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

if (( CONTRACT_ONLY == 1 )); then
  REQUEST_ID_A="not-issued"
  REQUEST_ID_B="not-issued"
  write_contract_artifacts
  exit 0
fi

ensure_secure_artifact_root
require_auth_boundary
source "$ROOT_DIR/scripts/agent-browser-lib.sh"
ab_require

# agent-browser launch configuration participates in the session identity.
# Export it for every command so a headed session is not silently relaunched
# headless/about:blank after the initial open.
export AGENT_BROWSER_HEADED=1
if [[ -z "${AGENT_BROWSER_ARGS:-}" ]]; then
  AGENT_BROWSER_ARGS="$(ab_browser_args),--no-proxy-server"
elif [[ ",$AGENT_BROWSER_ARGS," != *,--no-proxy-server,* ]]; then
  AGENT_BROWSER_ARGS+=",--no-proxy-server"
fi
export AGENT_BROWSER_ARGS

SESSION="$(ab_session_begin "viewer-prompt-control-race-${CASE_ID}-${RUN_ID}" "$OUT_DIR")"

if [[ -z "$GAME_URL" ]]; then
  echo "error: live dual-actor lane requires --url from the authoritative launcher" >&2
  exit 2
fi

require_loopback_url() {
  python3 - "$1" <<'PY'
import ipaddress
import sys
from urllib.parse import urlsplit
host = (urlsplit(sys.argv[1]).hostname or "").strip().lower()
if host == "localhost":
    raise SystemExit(0)
try:
    raise SystemExit(0 if ipaddress.ip_address(host).is_loopback else 1)
except ValueError:
    raise SystemExit(1)
PY
}
require_loopback_url "$GAME_URL" || { echo "error: live race URL must be loopback" >&2; exit 2; }
GAME_URL="$(python3 - "$GAME_URL" <<'PY'
import sys
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
parts = urlsplit(sys.argv[1])
query = dict(parse_qsl(parts.query, keep_blank_values=True))
query.update({"test_api": "1", "hosted_test_login": "1", "render_mode": "viewer"})
print(urlunsplit((parts.scheme, parts.netloc, parts.path, urlencode(query), parts.fragment)))
PY
)"

active_tab_id() {
  ab_read_retry "$SESSION" tab list --json | python3 -c '
import json, sys
data = json.load(sys.stdin)
payload = data.get("data", data) if isinstance(data, dict) else data
tabs = payload.get("tabs", payload if isinstance(payload, list) else [])
active = next((tab for tab in tabs if tab.get("active") or tab.get("isActive")), None)
if not active:
    raise SystemExit("active tab unavailable")
print(active.get("targetId") or active.get("tabId") or active.get("id") or "")
'
}

select_tab() {
  ab_run "$SESSION" tab "$1" >/dev/null
}

wait_js() {
  local expression="$1"
  local attempts=100
  local value
  while (( attempts > 0 )); do
    value="$(ab_read_retry "$SESSION" eval "$expression" 2>/dev/null || true)"
    case "$value" in true|\"true\") return 0 ;; esac
    sleep 0.2
    attempts=$((attempts - 1))
  done
  return 1
}

# Browser opens are action-bearing and intentionally use ab_open directly.
ab_open "$SESSION" 1 "$GAME_URL"
TAB_A="$(active_tab_id)"
wait_js 'typeof window.__AW_TEST__ === "object"' || { echo "error: actor A test API unavailable" >&2; exit 1; }
ab_run "$SESSION" click '[data-auth-action="test-login"]' >/dev/null
wait_js 'window.__AW_TEST__.getState()?.authReady === true' || { echo "error: actor A hosted test login unavailable" >&2; exit 1; }
ACTOR_B_URL="$(python3 - "$GAME_URL" <<'PY'
import sys
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
parts = urlsplit(sys.argv[1])
query = dict(parse_qsl(parts.query, keep_blank_values=True))
query.update({"connect": "0", "hosted_bootstrap": "0"})
print(urlunsplit((parts.scheme, parts.netloc, parts.path, urlencode(query), parts.fragment)))
PY
)"
ab_run "$SESSION" tab new "$ACTOR_B_URL" >/dev/null
TAB_B="$(active_tab_id)"
wait_js 'typeof window.__AW_TEST__ === "object"' || { echo "error: actor B test API unavailable" >&2; exit 1; }

# Reads and diagnostics may use the read-only retry helper. Action-bearing
# operations below use direct agent-browser commands exactly once.
read_state() {
  local tab_id="$1"
  select_tab "$tab_id"
  ab_read_retry "$SESSION" eval --stdin <<'JS'
window.__AW_TEST__?.getState?.() ?? null
JS
}

json_field() {
  local raw_json="$1"
  local expression="$2"
  python3 - "$raw_json" "$expression" <<'PY'
import json
import sys
data = json.loads(sys.argv[1])
value = data
for part in sys.argv[2].split("."):
    value = value.get(part) if isinstance(value, dict) else None
if value is None:
    print("")
elif isinstance(value, bool):
    print("true" if value else "false")
elif isinstance(value, (dict, list)):
    print(json.dumps(value, separators=(",", ":")))
else:
    print(value)
PY
}

prepare_actor() {
  local tab_id="$1"
  local goal="$2"
  local actor_label="$3"
  local artifact_dir="$4"
  local agent_json
  local starter_oc_visible
  local ready_to_continue
  agent_json="$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$AGENT_ID")"
  select_tab "$tab_id"
  wait_js 'window.__AW_TEST__?.getState?.()?.connectionStatus === "connected"' \
    || {
      write_safe_state "$(read_state "$tab_id")" "$artifact_dir/connection-failure-state-safe.json"
      echo "error: actor ${actor_label} connection did not become ready" >&2
      return 1
    }
  ab_run "$SESSION" eval "window.__AW_TEST__.registerPlayerSessionForTest(${agent_json})" >/dev/null
  wait_js "(() => { const s = window.__AW_TEST__.getState(); return s?.authRegistrationStatus === 'registered' && s?.authBoundAgentId === ${agent_json} && s?.authSessionEpoch != null && s?.authBindingEpoch != null; })()" \
    || {
      write_safe_state "$(read_state "$tab_id")" "$artifact_dir/binding-failure-state-safe.json"
      echo "error: actor ${actor_label} binding did not become ready" >&2
      return 1
    }
  ab_run "$SESSION" eval "window.__AW_TEST__.select('agent', ${agent_json}); true" >/dev/null
  wait_js "window.__AW_TEST__.getState()?.selectedId === ${agent_json}" \
    || { echo "error: actor ${actor_label} agent selection did not become ready" >&2; return 1; }
  starter_oc_visible="$(ab_read_retry "$SESSION" eval 'Boolean(document.querySelector("[data-viewer-fixture-state=starter_oc_required_gate]"))' 2>/dev/null || true)"
  case "$starter_oc_visible" in
    true|"true")
      ab_run "$SESSION" click '[data-testid="viewer-playthrough-action-claim-starter-oc"]' >/dev/null
      wait_js '(() => { const gate = document.querySelector("[data-viewer-fixture-state=starter_oc_required_gate]"); if (!gate) return true; const button = gate.querySelector("button[data-testid=viewer-playthrough-action-claim-starter-oc], button"); const label = String(button?.textContent || "").trim(); return /^(Continue|继续|Start First Agent Chat|开始第一次 Agent 聊天)$/.test(label); })()' || true
      ready_to_continue="$(ab_read_retry "$SESSION" eval '(() => { const gate = document.querySelector("[data-viewer-fixture-state=starter_oc_required_gate]"); if (!gate) return false; const button = gate.querySelector("button[data-testid=viewer-playthrough-action-claim-starter-oc], button"); return /^(Continue|继续|Start First Agent Chat|开始第一次 Agent 聊天)$/.test(String(button?.textContent || "").trim()); })()' 2>/dev/null || true)"
      case "$ready_to_continue" in
        true|"true") ab_run "$SESSION" click '[data-viewer-fixture-state="starter_oc_required_gate"] button' >/dev/null ;;
      esac
      wait_js '!document.querySelector("[data-viewer-fixture-state=starter_oc_required_gate]")' \
        || {
          write_safe_state "$(read_state "$tab_id")" "$artifact_dir/starter-oc-failure-state-safe.json"
          echo "error: actor ${actor_label} starter OC gate did not dismiss" >&2
          return 1
        }
      ;;
  esac
  ab_run "$SESSION" eval "window.__AW_TEST__.setPromptOverridesVisible(true); true" >/dev/null
  wait_js 'Boolean(document.querySelector("#prompt-short")) && Boolean(document.querySelector("#strong-auth-approval-code")) && Boolean(document.querySelector("button[data-prompt-action=apply]"))' \
    || { echo "error: actor prompt surface did not become ready" >&2; return 1; }
  ab_run "$SESSION" fill '#strong-auth-approval-code' "$OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE" >/dev/null
  ab_run "$SESSION" fill '#prompt-short' "$goal" >/dev/null
}

settle_starter_oc_gate() {
  local tab_id="$1"
  local actor_label="$2"
  local artifact_dir="$3"
  local visible
  select_tab "$tab_id"
  sleep 1
  visible="$(ab_read_retry "$SESSION" eval 'Boolean(document.querySelector("[data-viewer-fixture-state=starter_oc_required_gate]"))' 2>/dev/null || true)"
  case "$visible" in
    true|"true")
      ab_run "$SESSION" click '[data-testid="viewer-playthrough-action-claim-starter-oc"]' >/dev/null
      wait_js '!document.querySelector("[data-viewer-fixture-state=starter_oc_required_gate]")' || {
        write_safe_state "$(read_state "$tab_id")" "$artifact_dir/starter-oc-settle-failure-state-safe.json"
        echo "error: actor ${actor_label} late starter OC gate did not dismiss" >&2
        return 1
      }
      ;;
  esac
}

timestamp_ns() {
  python3 -c 'import time; print(time.time_ns())'
}

write_dispatch_blocked() {
  local reason="$1"
  python3 - "$OUT_DIR/dispatch-trace.json" "$reason" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.parent.chmod(0o700)
path.write_text(json.dumps({
    "dispatchEligible": False,
    "dispatchMode": "blocked",
    "reason": sys.argv[2],
    "sameOwnedBrowserSession": True,
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

init_pinned_dispatch_handles() {
  local session_info
  session_info="$(ab_read_retry "$SESSION" session info --json 2>/dev/null || true)"
  CDP_URL="$(json_field "$session_info" data.cdpUrl)"
  [[ -n "$CDP_URL" ]] || CDP_URL="$(json_field "$session_info" data.cdp_url)"
  [[ -n "$CDP_URL" ]] || CDP_URL="$(json_field "$session_info" cdpUrl)"
  [[ -n "$CDP_URL" ]] || CDP_URL="$(json_field "$session_info" cdp_url)"
  if [[ -z "$CDP_URL" ]]; then
    write_dispatch_blocked "owned browser session did not expose a CDP endpoint"
    return 1
  fi

  # agent-browser maintains one active tab per session.  Use two short-lived
  # pinned CDP control handles against the same owned Chrome so each visible
  # click is bound to a stable target without racing tab selection.
  APPLY_HANDLE_A="${SESSION}-race-a"
  APPLY_HANDLE_B="${SESSION}-race-b"
  if ! ab_run "$APPLY_HANDLE_A" --cdp "$CDP_URL" --pin-tab tab "$TAB_A" >/dev/null 2>&1; then
    write_dispatch_blocked "failed to pin actor A CDP handle to its tab"
    return 1
  fi
  if ! ab_run "$APPLY_HANDLE_B" --cdp "$CDP_URL" --pin-tab tab "$TAB_B" >/dev/null 2>&1; then
    write_dispatch_blocked "failed to pin actor B CDP handle to its tab"
    return 1
  fi
  return 0
}

dispatch_apply_actor_a() {
  ab_run "$APPLY_HANDLE_A" --cdp "$CDP_URL" --pin-tab --json click 'button[data-prompt-action="apply"]'
}

dispatch_apply_actor_b() {
  ab_run "$APPLY_HANDLE_B" --cdp "$CDP_URL" --pin-tab --json click 'button[data-prompt-action="apply"]'
}

write_dispatch_receipt() {
  local actor="$1"
  local target_id="$2"
  local ready_at_ns="$3"
  local started_at_ns="$4"
  local ended_at_ns="$5"
  local status="$6"
  local output="$7"
  local path="$8"
  python3 - "$path" "$actor" "$target_id" "$ready_at_ns" "$started_at_ns" "$ended_at_ns" "$status" "$output" <<'PY'
import hashlib
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.parent.chmod(0o700)
output = sys.argv[8].encode("utf-8", errors="replace")
path.write_text(json.dumps({
    "actor": sys.argv[2],
    "command": "agent-browser --pin-tab click button[data-prompt-action=apply]",
    "endedAtNs": int(sys.argv[6]),
    "exitCode": int(sys.argv[7]),
    "outputBytes": len(output),
    "outputSha256": hashlib.sha256(output).hexdigest(),
    "readyAtNs": int(sys.argv[4]),
    "startedAtNs": int(sys.argv[5]),
    "status": "passed" if int(sys.argv[7]) == 0 else "failed",
    "targetId": sys.argv[3],
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

dispatch_apply_worker() {
  local actor="$1"
  local handle="$2"
  local target_id="$3"
  local ready_path="$4"
  local receipt_path="$5"
  local ready_at_ns started_at_ns ended_at_ns status output
  ready_at_ns="$(timestamp_ns)"
  printf '%s\n' "$ready_at_ns" >"${ready_path}.tmp.$$"
  mv -f "${ready_path}.tmp.$$" "$ready_path"
  while [[ ! -f "$DISPATCH_RELEASE" ]]; do
    sleep 0.01
  done
  started_at_ns="$(timestamp_ns)"
  if [[ "$actor" == "A" ]]; then
    if output="$(dispatch_apply_actor_a 2>&1)"; then
      status=0
    else
      status=$?
    fi
  else
    if output="$(dispatch_apply_actor_b 2>&1)"; then
      status=0
    else
      status=$?
    fi
  fi
  ended_at_ns="$(timestamp_ns)"
  write_dispatch_receipt "$actor" "$target_id" "$ready_at_ns" "$started_at_ns" \
    "$ended_at_ns" "$status" "$output" "$receipt_path"
  return "$status"
}

write_dispatch_trace() {
  python3 - "$OUT_DIR/dispatch-trace.json" "$DISPATCH_RELEASE_AT_NS" \
    "$DISPATCH_RECEIPT_A" "$DISPATCH_RECEIPT_B" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
release_at_ns = int(sys.argv[2]) if sys.argv[2].isdigit() else 0
def read_receipt(raw_path):
    try:
        value = json.loads(pathlib.Path(raw_path).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {"status": "missing"}
    return value if isinstance(value, dict) else {"status": "invalid"}

actors = [read_receipt(sys.argv[3]), read_receipt(sys.argv[4])]
starts = [value.get("startedAtNs") for value in actors if isinstance(value.get("startedAtNs"), int)]
ends = [value.get("endedAtNs") for value in actors if isinstance(value.get("endedAtNs"), int)]
window_ns = max(starts) - min(starts) if len(starts) == 2 else None
overlap = len(starts) == 2 and len(ends) == 2 and max(starts) < min(ends)
dispatch_eligible = (
    len(actors) == 2
    and all(value.get("status") == "passed" for value in actors)
    and len(starts) == 2
    and release_at_ns > 0
    and min(starts) >= release_at_ns
    and window_ns is not None
    and window_ns <= 250_000_000
)
path.parent.mkdir(parents=True, exist_ok=True)
path.parent.chmod(0o700)
path.write_text(json.dumps({
    "actorReceipts": actors,
    "dispatchEligible": dispatch_eligible,
    "dispatchMode": "same-owned-chrome-cdp-pinned-tabs",
    "dispatchStartWindowNs": window_ns,
    "releaseAtNs": release_at_ns,
    "sameOwnedBrowserSession": True,
    "visibleAction": "button[data-prompt-action=apply]",
    "workersOverlap": overlap,
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
if not dispatch_eligible:
    raise SystemExit("parallel pinned-tab dispatch was not proven")
PY
}

dispatch_apply_race() {
  local attempts=1000
  local status_a=0
  local status_b=0
  DISPATCH_DIR="$OUT_DIR/dispatch"
  DISPATCH_RELEASE="$DISPATCH_DIR/release"
  DISPATCH_READY_A="$DISPATCH_DIR/actor-a.ready"
  DISPATCH_READY_B="$DISPATCH_DIR/actor-b.ready"
  DISPATCH_RECEIPT_A="$DISPATCH_DIR/actor-a.json"
  DISPATCH_RECEIPT_B="$DISPATCH_DIR/actor-b.json"
  mkdir -p "$DISPATCH_DIR"
  chmod 700 "$DISPATCH_DIR"
  : >"$DISPATCH_RELEASE"
  : >"$DISPATCH_READY_A"
  : >"$DISPATCH_READY_B"
  rm -f "$DISPATCH_RECEIPT_A" "$DISPATCH_RECEIPT_B"

  dispatch_apply_worker "A" "$APPLY_HANDLE_A" "$TAB_A" "$DISPATCH_READY_A" \
    "$DISPATCH_RECEIPT_A" >"$DISPATCH_DIR/actor-a.log" 2>&1 &
  DISPATCH_PID_A=$!
  dispatch_apply_worker "B" "$APPLY_HANDLE_B" "$TAB_B" "$DISPATCH_READY_B" \
    "$DISPATCH_RECEIPT_B" >"$DISPATCH_DIR/actor-b.log" 2>&1 &
  DISPATCH_PID_B=$!
  while (( attempts > 0 )) && { [[ ! -s "$DISPATCH_READY_A" ]] || [[ ! -s "$DISPATCH_READY_B" ]]; }; do
    sleep 0.01
    attempts=$((attempts - 1))
  done
  if [[ ! -s "$DISPATCH_READY_A" || ! -s "$DISPATCH_READY_B" ]]; then
    write_dispatch_blocked "both pinned-tab workers did not reach the dispatch barrier"
    kill "$DISPATCH_PID_A" "$DISPATCH_PID_B" >/dev/null 2>&1 || true
    wait "$DISPATCH_PID_A" >/dev/null 2>&1 || true
    wait "$DISPATCH_PID_B" >/dev/null 2>&1 || true
    return 1
  fi
  DISPATCH_RELEASE_AT_NS="$(timestamp_ns)"
  printf '%s\n' "$DISPATCH_RELEASE_AT_NS" >"$DISPATCH_RELEASE"
  wait "$DISPATCH_PID_A" || status_a=$?
  wait "$DISPATCH_PID_B" || status_b=$?
  write_dispatch_trace || return 1
  if (( status_a != 0 || status_b != 0 )); then
    return 1
  fi
  return 0
}

if [[ -z "${OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE:-}" ]]; then
  echo "error: OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE is required" >&2
  exit 2
fi

prepare_actor "$TAB_A" "Race actor A ${RUN_ID}" "A" "$ARTIFACT_DIR_A"
select_tab "$TAB_A"
handoff_descriptor="$(ab_read_retry "$SESSION" eval 'window.__AW_TEST__.offerBrowserRaceIdentityForTest()')"
descriptor_json="$(python3 - "$handoff_descriptor" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
print(json.dumps(value, separators=(",", ":")))
PY
)"
select_tab "$TAB_B"
ab_run "$SESSION" eval "window.__AW_TEST__.claimBrowserRaceIdentityForTest(${descriptor_json})" >/dev/null
ab_run "$SESSION" eval 'window.__AW_TEST__.connectBrowserRaceActorForTest()' >/dev/null
prepare_actor "$TAB_B" "Race actor B ${RUN_ID}" "B" "$ARTIFACT_DIR_B"
settle_starter_oc_gate "$TAB_A" "A" "$ARTIFACT_DIR_A"
settle_starter_oc_gate "$TAB_B" "B" "$ARTIFACT_DIR_B"

baseline_a="$(read_state "$TAB_A")"
baseline_b="$(read_state "$TAB_B")"
baseline_version="$(json_field "$baseline_a" selectedPromptVersion)"
baseline_version_b="$(json_field "$baseline_b" selectedPromptVersion)"
baseline_session="$(json_field "$baseline_a" authSessionEpoch)"
baseline_binding="$(json_field "$baseline_a" authBindingEpoch)"
baseline_authority="$(json_field "$baseline_a" authAuthorityEpoch)"
baseline_player="$(json_field "$baseline_a" authPlayerId)"
baseline_public_key="$(json_field "$baseline_a" authPublicKey)"
baseline_agent="$(json_field "$baseline_a" authBoundAgentId)"
baseline_selected="$(json_field "$baseline_a" selectedId)"
same_baseline=false
if [[ -n "$baseline_version" \
  && "$baseline_version" == "$baseline_version_b" \
  && "$baseline_session" == "$(json_field "$baseline_b" authSessionEpoch)" \
  && "$baseline_binding" == "$(json_field "$baseline_b" authBindingEpoch)" \
  && -n "$baseline_authority" \
  && "$baseline_authority" == "$(json_field "$baseline_b" authAuthorityEpoch)" \
  && -n "$baseline_player" \
  && "$baseline_player" == "$(json_field "$baseline_b" authPlayerId)" \
  && -n "$baseline_public_key" \
  && "$baseline_public_key" == "$(json_field "$baseline_b" authPublicKey)" \
  && "$baseline_agent" == "$AGENT_ID" \
  && "$baseline_agent" == "$(json_field "$baseline_b" authBoundAgentId)" \
  && "$baseline_selected" == "$AGENT_ID" \
  && "$baseline_selected" == "$(json_field "$baseline_b" selectedId)" ]]; then
  same_baseline=true
fi
if [[ "$same_baseline" != true ]]; then
  echo "error: actors do not share one authoritative prompt baseline" >&2
  exit 1
fi

# Freeze the expected version from the common baseline.  No state/version read
# is allowed after actor A's action has been issued: the two signed requests
# must carry the same frozen expected_version rather than a post-commit read.
actor_a_expected_version="$baseline_version"
actor_b_expected_version="$baseline_version"
init_pinned_dispatch_handles || exit 1
dispatch_apply_race || {
  echo "error: live race dispatch was not proven concurrent; acceptance is blocked" >&2
  exit 1
}

wait_actor_terminal() {
  local tab_id="$1"
  select_tab "$tab_id"
  wait_js '(() => { const f = window.__AW_TEST__.getState()?.lastPromptFeedback; return f?.action === "prompt_apply" && ["applied", "stale"].includes(f?.stage); })()'
}
wait_actor_terminal "$TAB_A" || { echo "error: actor A Apply result timed out" >&2; exit 1; }
wait_actor_terminal "$TAB_B" || { echo "error: actor B Apply result timed out" >&2; exit 1; }

state_a="$(read_state "$TAB_A")"
state_b="$(read_state "$TAB_B")"
outcome_a="$(json_field "$state_a" lastPromptFeedback.stage)"
outcome_b="$(json_field "$state_b" lastPromptFeedback.stage)"
REQUEST_ID_A="$(json_field "$state_a" lastPromptFeedback.requestId)"
REQUEST_ID_B="$(json_field "$state_b" lastPromptFeedback.requestId)"
reason_a="$(json_field "$state_a" lastPromptFeedback.response.reason_code)"
reason_b="$(json_field "$state_b" lastPromptFeedback.response.reason_code)"
response_a="$(json_field "$state_a" lastPromptFeedback.response)"
response_b="$(json_field "$state_b" lastPromptFeedback.response)"

python3 - "$OUT_DIR/race-results.json" "$baseline_version" "$baseline_session" "$baseline_binding" "$baseline_authority" "$baseline_player" "$baseline_agent" "$baseline_public_key" "$actor_a_expected_version" "$actor_b_expected_version" "$outcome_a" "$outcome_b" "$REQUEST_ID_A" "$REQUEST_ID_B" "$reason_a" "$reason_b" "$response_a" "$response_b" "$OUT_DIR/dispatch-trace.json" <<'PY'
import hashlib
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
baseline_version = int(sys.argv[2])
baseline_session = int(sys.argv[3])
baseline_binding = int(sys.argv[4])
baseline_authority = sys.argv[5]
baseline_player = sys.argv[6]
baseline_agent = sys.argv[7]
baseline_public_key = sys.argv[8]
expected_versions = [sys.argv[9], sys.argv[10]]
outcomes = [sys.argv[11], sys.argv[12]]
counts = {"applied": outcomes.count("applied"), "stale": outcomes.count("stale")}
request_ids = [sys.argv[13], sys.argv[14]]
reasons = [sys.argv[15], sys.argv[16]]
def parse_response(raw):
    try:
        value = json.loads(raw)
    except (TypeError, ValueError):
        value = {}
    return value if isinstance(value, dict) else {}

responses = [parse_response(sys.argv[17]), parse_response(sys.argv[18])]
dispatch = {}
try:
    value = json.loads(pathlib.Path(sys.argv[19]).read_text(encoding="utf-8"))
    dispatch = value if isinstance(value, dict) else {}
except (OSError, ValueError):
    pass
def int_or_none(value):
    return value if isinstance(value, int) and not isinstance(value, bool) else None

statuses = [response.get("status") for response in responses]
response_request_ids = [response.get("request_id") for response in responses]
response_versions = [
    int_or_none(response.get("version"))
    if int_or_none(response.get("version")) is not None
    else int_or_none(response.get("current_version"))
    for response in responses
]
response_expected_versions = [int_or_none(response.get("expected_version")) for response in responses]
response_authorities = [response.get("authority_epoch") for response in responses]
response_agents = [response.get("agent_id") for response in responses]
response_players = [response.get("player_id") for response in responses]
response_sessions = [int_or_none(response.get("session_epoch")) for response in responses]
response_bindings = [int_or_none(response.get("binding_epoch")) for response in responses]
mutations = [int_or_none(response.get("mutation_count")) for response in responses]
operation_digests = [response.get("operation_digest") for response in responses]
applied_index = statuses.index("applied") if "applied" in statuses else -1
stale_index = outcomes.index("stale") if "stale" in outcomes else -1
eligible = (
    expected_versions == [str(baseline_version), str(baseline_version)]
    and counts == {"applied": 1, "stale": 1}
    and all(request_ids)
    and request_ids[0] != request_ids[1]
    and request_ids == response_request_ids
    and statuses == outcomes
    and response_versions == [baseline_version + 1, baseline_version + 1]
    and response_expected_versions == [baseline_version, baseline_version]
    and stale_index >= 0
    and reasons[stale_index] == "version_conflict"
    and mutations[stale_index] in (None, 0)
    and applied_index >= 0
    and mutations[applied_index] == 1
    and responses[applied_index].get("applied_scope") == "runtime_instance"
    and responses[applied_index].get("persistence_scope") == "none"
    and responses[applied_index].get("sync_scope") == "none"
    and all(
        response.get("applied_scope") in (None, "none")
        and response.get("persistence_scope") in (None, "none")
        and response.get("sync_scope") in (None, "none")
        for index, response in enumerate(responses)
        if index == stale_index
    )
    and all(response_authority == baseline_authority for response_authority in response_authorities)
    and all(response_agent == baseline_agent for response_agent in response_agents)
    and all(response_player == baseline_player for response_player in response_players)
    and all(response_session == baseline_session for response_session in response_sessions)
    and all(response_binding == baseline_binding for response_binding in response_bindings)
    and all(isinstance(digest, str) and bool(digest) for digest in operation_digests)
    and all(response.get("idempotent_replay") is not True for response in responses)
    and dispatch.get("dispatchEligible") is True
)
result = {
    "acceptanceEligible": eligible,
    "actors": [
        {"label": "A", "artifactDir": "actor-a", "request_id": request_ids[0]},
        {"label": "B", "artifactDir": "actor-b", "request_id": request_ids[1]},
    ],
    "baseline": {
        "authorityEpoch": baseline_authority,
        "agentId": baseline_agent,
        "bindingEpoch": baseline_binding,
        "keyFingerprint": hashlib.sha256(baseline_public_key.encode("utf-8")).hexdigest(),
        "playerFingerprint": hashlib.sha256(baseline_player.encode("utf-8")).hexdigest(),
        "same_baseline": eligible or (
            expected_versions == [str(baseline_version), str(baseline_version)]
            and all(response_authority == baseline_authority for response_authority in response_authorities)
            and all(response_agent == baseline_agent for response_agent in response_agents)
            and all(response_player == baseline_player for response_player in response_players)
        ),
        "sessionEpoch": baseline_session,
        "version": baseline_version,
    },
    "dispatch": dispatch,
    "evidenceTier": "real_browser_same_identity_two_tab_race",
    "expected_version": baseline_version,
    "outcomeCounts": counts,
    "requestIdsDistinct": request_ids[0] != request_ids[1],
    "runtimeActionsDuringVerification": [
        "hosted_test_login",
        "player_session_registration",
        "same_identity_handoff",
        "visible_prompt_apply_actor_a",
        "visible_prompt_apply_actor_b",
    ],
    "responses": [
        {
            "actor": label,
            "status": response.get("status"),
            "request_id": response.get("request_id"),
            "version": response_versions[index],
            "current_version": int_or_none(response.get("current_version")),
            "mutation_count": mutations[index],
            "authority_epoch": response.get("authority_epoch"),
            "session_epoch": response_sessions[index],
            "binding_epoch": response_bindings[index],
            "applied_scope": response.get("applied_scope"),
            "persistence_scope": response.get("persistence_scope"),
            "sync_scope": response.get("sync_scope"),
            "operation_digest": response.get("operation_digest"),
            "idempotent_replay": response.get("idempotent_replay", False),
        }
        for index, (label, response) in enumerate(zip(("A", "B"), responses))
    ],
}
path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
if not eligible:
    raise SystemExit("race result did not prove concurrent exactly-one applied/stale authority receipts")
PY

write_safe_state "$state_a" "$ARTIFACT_DIR_A/state-safe.json"
write_safe_state "$state_b" "$ARTIFACT_DIR_B/state-safe.json"
write_actor_receipt "A" "$ARTIFACT_DIR_A" "$SESSION" "$REQUEST_ID_A" "$outcome_a" "$reason_a" "$actor_a_expected_version" "$response_a" "$baseline_player"
write_actor_receipt "B" "$ARTIFACT_DIR_B" "$SESSION" "$REQUEST_ID_B" "$outcome_b" "$reason_b" "$actor_b_expected_version" "$response_b" "$baseline_player"
echo "headed same-identity two-tab prompt race complete (manifest: $OUT_DIR/artifact-manifest.json)"
