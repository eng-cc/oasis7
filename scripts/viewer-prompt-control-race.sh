#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASE_ID="PWT-004"
OUT_DIR="output/playwright/prompt-control/${CASE_ID}-race"
CONTRACT_ONLY=0
LIVE=0
HEADED=0
GAME_URL=""
AUTH_BOUNDARY_PROOF=""
RUN_ID="viewer-prompt-control-race-$(date -u +%Y%m%dT%H%M%SZ)-$$"

# Explicit actor/session names are required by the race evidence contract.
# They stay empty until the fail-closed live gate passes.
SESSION_A=""
SESSION_B=""
ARTIFACT_DIR_A=""
ARTIFACT_DIR_B=""
REQUEST_ID_A=""
REQUEST_ID_B=""

usage() {
  cat <<'EOF'
Usage: viewer-prompt-control-race.sh --contract-only

The live two-actor lane is fail-closed until runtime-owned auth-boundary
proof is supplied. Contract-only mode performs no browser/provider calls.

Options:
  --contract-only
  --live --headed --url URL --auth-boundary-proof PATH
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
out_path.write_text(json.dumps(redact(data), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}

write_actor_receipt() {
  local actor_label="$1"
  local artifact_dir="$2"
  local session="$3"
  local request_id="$4"
  python3 - "$artifact_dir/actor-receipt.json" "$actor_label" "$session" "$request_id" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps({
    "actor": sys.argv[2],
    "request_id": sys.argv[4],
    "session": "owned-session-redacted" if sys.argv[3] else "not-started",
    "promptSubmission": "not-issued",
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
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
    "acceptanceEligible": False,
    "actors": [
        {"label": "A", "artifactDir": "actor-a", "request_id": "not-issued"},
        {"label": "B", "artifactDir": "actor-b", "request_id": "not-issued"},
    ],
    "artifacts": artifacts,
    "authBoundary": {"eligibility": "not-asserted", "proofSupplied": bool(proof)},
    "browserMode": "headed",
    "caseId": case_id,
    "evidenceTier": "contract_only",
    "providerCallsDuringVerification": False,
    "schema": "oasis7.viewer.prompt-control-race-artifact-manifest/v1",
    "visibleActionContract": {"promptSubmission": "not-issued"},
}
(root / "artifact-manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY
}

write_contract_artifacts() {
  mkdir -p "$OUT_DIR" "$ARTIFACT_DIR_A" "$ARTIFACT_DIR_B"
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
    "sessionSurface": {
        "actorA": {"session": "SESSION_A", "artifactDir": "actor-a"},
        "actorB": {"session": "SESSION_B", "artifactDir": "actor-b"},
    },
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
  write_manifest
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

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$SESSION_A" ]]; then
    ab_session_cleanup "$SESSION_A" "$ARTIFACT_DIR_A" || true
  fi
  if [[ -n "$SESSION_B" ]]; then
    ab_session_cleanup "$SESSION_B" "$ARTIFACT_DIR_B" || true
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

require_auth_boundary
source "$ROOT_DIR/scripts/agent-browser-lib.sh"
ab_require
mkdir -p "$OUT_DIR" "$ARTIFACT_DIR_A" "$ARTIFACT_DIR_B"

SESSION_A="$(ab_session_begin "viewer-prompt-control-race-${CASE_ID}-actor-a-${RUN_ID}" "$ARTIFACT_DIR_A")"
SESSION_B="$(ab_session_begin "viewer-prompt-control-race-${CASE_ID}-actor-b-${RUN_ID}" "$ARTIFACT_DIR_B")"

if [[ -z "$GAME_URL" ]]; then
  echo "error: live dual-actor lane requires --url from the authoritative launcher" >&2
  exit 2
fi

# Browser opens are action-bearing and intentionally use ab_open directly.
ab_open "$SESSION_A" 1 "$GAME_URL"
ab_open "$SESSION_B" 1 "$GAME_URL"

# Reads and diagnostics may use the read-only retry helper. No prompt
# operation is issued until the auth owner supplies a concrete race protocol.
read_state() {
  local session="$1"
  ab_read_retry "$session" eval --stdin <<'JS'
window.__AW_TEST__?.getState?.() ?? null
JS
}

REQUEST_ID_A="${RUN_ID}-actor-a-not-issued"
REQUEST_ID_B="${RUN_ID}-actor-b-not-issued"
state_a="$(read_state "$SESSION_A")"
state_b="$(read_state "$SESSION_B")"
write_safe_state "$state_a" "$ARTIFACT_DIR_A/state-safe.json"
write_safe_state "$state_b" "$ARTIFACT_DIR_B/state-safe.json"
write_actor_receipt "A" "$ARTIFACT_DIR_A" "$SESSION_A" "$REQUEST_ID_A"
write_actor_receipt "B" "$ARTIFACT_DIR_B" "$SESSION_B" "$REQUEST_ID_B"
write_manifest
echo "race live setup completed without prompt submission; acceptance remains blocked on auth-boundary ownership"
