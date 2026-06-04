#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

CREDENTIALS_FILE="${ALIYUN_ECS_CREDENTIALS_FILE:-/Users/scc/Documents/keys/aliyun_ecs.txt}"
TEST_HOST="39.104.204.172"
PROD_HOST="39.104.205.67"
PUBLIC_BASE_URL="${OASIS7_PROVIDER_PUBLIC_BASE_URL:-https://t2t.oasis7.tech}"
DECISION_COUNT="${OASIS7_PROVIDER_LIVE_DECISION_COUNT:-1}"
TIMEOUT_MS="${OASIS7_PROVIDER_LIVE_TIMEOUT_MS:-15000}"
RUN_PUBLIC="1"
RUN_LOOPBACK="1"
LOOPBACK_TARGET="${OASIS7_PROVIDER_LIVE_LOOPBACK_TARGET:-both}"
LOWQUOTA_TARGET=""
LOWQUOTA_DECISION_COUNT="${OASIS7_PROVIDER_LIVE_LOWQUOTA_DECISION_COUNT:-20}"
LOWQUOTA_EXPECT_SUBSTR="${OASIS7_PROVIDER_LIVE_LOWQUOTA_EXPECT_SUBSTR:-quota}"
RUN_ACCOUNTING="0"
ACCOUNTING_HOST="$TEST_HOST"
ACCOUNTING_BIND_PAYLOAD_FILE="${OASIS7_PROVIDER_LIVE_ACCOUNTING_BIND_PAYLOAD_FILE:-}"
ACCOUNTING_PRICING_VERSION="${OASIS7_PROVIDER_LIVE_ACCOUNTING_PRICING_VERSION:-pv-1}"
ACCOUNTING_TRANSFER_COMMAND="${OASIS7_PROVIDER_LIVE_ACCOUNTING_TRANSFER_COMMAND:-}"
ACCOUNTING_ALLOW_MUTATION="${OASIS7_PROVIDER_LIVE_ACCOUNTING_ALLOW_MUTATION:-0}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/provider-remote-https/provider-bridge-live-gate.sh [options]

Runs real provider bridge checks against live ECS environments. This script
does not print raw token_key, raw bridge state, private keys, or passwords.

Default checks:
  - SSH to 39.104.204.172 and 39.104.205.67.
  - Derive active newapi_user_ref bearer selectors from each host's real
    /etc/oasis7/newapi-bridge/bridge-state.json.
  - POST real provider decision smoke through each host's loopback provider.
  - POST real provider decision smoke through https://t2t.oasis7.tech using
    the 205 production selector.

Options:
  --credentials-file <path>       default: /Users/scc/Documents/keys/aliyun_ecs.txt
  --decision-count <n>            default: 1
  --timeout-ms <n>                default: 15000
  --skip-public                   skip https://t2t.oasis7.tech check
  --skip-loopback                 skip ECS loopback checks
  --loopback-target <204|205|both>
                                  default: both
  --lowquota-target <204|205|public205>
                                  run repeated live decisions until a provider_error
                                  code containing "quota" is observed
  --lowquota-decision-count <n>   default: 20
  --accounting-host <204|205>     host for live bind/deposit/reconcile accounting
  --accounting-bind-payload-file <path>
                                  JSON bind payload for a dedicated live persona
  --accounting-pricing-version <v> default: pv-1
  --accounting-transfer-command <cmd>
                                  command run locally after route creation; receives
                                  TEST_BRIDGE_USER_ID, TEST_DEPOSIT_ROUTE_ID,
                                  TEST_DEPOSIT_ACCOUNT_ID, TEST_BRIDGE_BASE_URL,
                                  TEST_PROVIDER_BASE_URL in env
  --i-understand-accounting-mutates-live-state
                                  required for accounting mode

Environment aliases mirror the option names with OASIS7_PROVIDER_LIVE_*.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --credentials-file)
      CREDENTIALS_FILE="${2:-}"
      shift 2
      ;;
    --decision-count)
      DECISION_COUNT="${2:-}"
      shift 2
      ;;
    --timeout-ms)
      TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --skip-public)
      RUN_PUBLIC="0"
      shift
      ;;
    --skip-loopback)
      RUN_LOOPBACK="0"
      shift
      ;;
    --loopback-target)
      LOOPBACK_TARGET="${2:-}"
      shift 2
      ;;
    --lowquota-target)
      LOWQUOTA_TARGET="${2:-}"
      shift 2
      ;;
    --lowquota-decision-count)
      LOWQUOTA_DECISION_COUNT="${2:-}"
      shift 2
      ;;
    --accounting-host)
      case "${2:-}" in
        204) ACCOUNTING_HOST="$TEST_HOST" ;;
        205) ACCOUNTING_HOST="$PROD_HOST" ;;
        *) echo "error: --accounting-host must be 204 or 205" >&2; exit 2 ;;
      esac
      RUN_ACCOUNTING="1"
      shift 2
      ;;
    --accounting-bind-payload-file)
      ACCOUNTING_BIND_PAYLOAD_FILE="${2:-}"
      RUN_ACCOUNTING="1"
      shift 2
      ;;
    --accounting-pricing-version)
      ACCOUNTING_PRICING_VERSION="${2:-}"
      shift 2
      ;;
    --accounting-transfer-command)
      ACCOUNTING_TRANSFER_COMMAND="${2:-}"
      RUN_ACCOUNTING="1"
      shift 2
      ;;
    --i-understand-accounting-mutates-live-state)
      ACCOUNTING_ALLOW_MUTATION="1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -f "$CREDENTIALS_FILE" ]] || { echo "error: credentials file not found: $CREDENTIALS_FILE" >&2; exit 2; }
[[ "$DECISION_COUNT" =~ ^[0-9]+$ && "$DECISION_COUNT" -gt 0 ]] || { echo "error: --decision-count must be positive" >&2; exit 2; }
[[ "$TIMEOUT_MS" =~ ^[0-9]+$ && "$TIMEOUT_MS" -gt 0 ]] || { echo "error: --timeout-ms must be positive" >&2; exit 2; }
[[ "$LOWQUOTA_DECISION_COUNT" =~ ^[0-9]+$ && "$LOWQUOTA_DECISION_COUNT" -gt 0 ]] || { echo "error: --lowquota-decision-count must be positive" >&2; exit 2; }
case "$LOOPBACK_TARGET" in
  204|205|both) ;;
  *) echo "error: --loopback-target must be 204, 205, or both" >&2; exit 2 ;;
esac

password_for_host() {
  local host="$1"
  awk -F: -v target="root@${host}" '$1 == target { print $2; found=1; exit } END { if (!found) exit 1 }' "$CREDENTIALS_FILE"
}

ssh_capture() {
  local host="$1"
  local command="$2"
  local password
  password="$(password_for_host "$host")" || { echo "error: no credential for root@$host" >&2; exit 2; }
  local command_b64
  command_b64="$(python3 -c 'import base64,sys; print(base64.b64encode(sys.stdin.read().encode()).decode())' <<<"$command")"
  local output_file
  output_file="$(mktemp)"
  local expect_status=0
  PASS="$password" HOST="$host" REMOTE_COMMAND_B64="$command_b64" OUT="$output_file" expect <<'EXPECT' || expect_status=$?
set timeout 60
set password $env(PASS)
set host $env(HOST)
set encoded $env(REMOTE_COMMAND_B64)
set command "python3 -c \"import base64,sys; sys.stdout.write(base64.b64decode('$encoded').decode())\" | bash"
set outfile $env(OUT)
set transcript ""
log_user 0
spawn ssh -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/Users/scc/.ssh/known_hosts root@$host $command
expect {
  "*assword:*" { send -- "$password\r"; exp_continue }
  -re {(.|\r|\n)+} { append transcript $expect_out(buffer); exp_continue }
  timeout { exit 124 }
  eof {}
}
set handle [open $outfile w]
puts -nonewline $handle $transcript
close $handle
catch wait result
set exit_code [lindex $result 3]
exit $exit_code
EXPECT
  if [[ "$expect_status" -ne 0 ]]; then
    sed -E 's/(Authorization: Bearer )[[:graph:]]+/\1<redacted>/g' "$output_file" >&2 || true
    rm -f "$output_file"
    return "$expect_status"
  fi
  sed -E 's/(Authorization: Bearer )[[:graph:]]+/\1<redacted>/g' "$output_file"
  rm -f "$output_file"
}

selector_for_host() {
  local host="$1"
  ssh_capture "$host" "python3 - <<'PY'
import json
from pathlib import Path
state = json.loads(Path('/etc/oasis7/newapi-bridge/bridge-state.json').read_text())
for binding in state.get('bindings', []):
    if binding.get('status') != 'active':
        continue
    ref = str(binding.get('newapi_user_ref') or '').strip()
    bridge_user_id = str(binding.get('bridge_user_id') or '').strip()
    has_token = any(
        p.get('bridge_user_id') == bridge_user_id and str(p.get('token_key') or '').strip()
        for p in state.get('project_bindings', [])
    )
    if ref and has_token:
        print('newapi_user_ref:' + ref)
        raise SystemExit(0)
raise SystemExit('no active binding with token_key')
PY" | tr -d '\r' | tail -n 1
}

run_contract_smoke() {
  local label="$1"
  local base_url="$2"
  local token="$3"
  local decision_count="$4"
  local min_successes="$5"
  shift 5
  echo "== ${label} =="
  python3 scripts/provider-remote-https/provider_bridge_contract_smoke.py \
    --base-url "$base_url" \
    --auth-token "$token" \
    --timeout-ms "$TIMEOUT_MS" \
    --decision-count "$decision_count" \
    --min-successes "$min_successes" \
    "$@"
}

run_loopback_host_smoke() {
  local host="$1"
  local selector="$2"
  local label="$3"
  local decision_count="$4"
  local min_successes="$5"
  local expected_error_substr="${6:-}"
  local remote_command
  # Copying the full Python harness to the host is unnecessary; invoke the
  # endpoint with curl and summarize the response with on-host Python.
  remote_command="AUTH_TOKEN=$(printf '%q' "$selector") DECISION_COUNT=$(printf '%q' "$decision_count") MIN_SUCCESSES=$(printf '%q' "$min_successes") EXPECTED_ERROR_SUBSTR=$(printf '%q' "$expected_error_substr") TIMEOUT_SECONDS=$(printf '%q' "$(( (TIMEOUT_MS + 999) / 1000 ))") python3 - <<'PY'
import json, os, subprocess, tempfile
auth = os.environ['AUTH_TOKEN']
count = int(os.environ['DECISION_COUNT'])
min_successes = int(os.environ['MIN_SUCCESSES'])
expected_error_substr = os.environ.get('EXPECTED_ERROR_SUBSTR') or ''
timeout = os.environ['TIMEOUT_SECONDS']
successes = 0
versions = []
errors = []
error_messages = []
for index in range(1, count + 1):
    payload = {
        'observation': {
            'agent_id': f'live-smoke-{index}',
            'world_time': index,
            'mode': 'headless_agent',
            'observation_schema_version': 'oc_dual_obs_v1',
            'action_schema_version': 'oc_dual_act_v1',
            'environment_class': 'provider_bridge_live_gate',
            'observation': {
                'self_state': {'location_ref': 'loc-1', 'pose_hint': 'grid_pose=(0, 0, 0)', 'status_flags': [], 'resource_summary': {}},
                'mission_context': {'goal_summary': 'return a minimal wait decision for live provider smoke'},
                'nearby_entities': [],
                'recent_events': [],
                'local_navigation_graph': [],
                'hazard_summary': [],
                'interaction_targets': [],
            },
            'recent_event_summary': [],
        'action_catalog': [{'action_ref': 'wait', 'summary': 'do nothing this tick'}],
            'timeout_budget_ms': 15000,
        },
        'provider_config_ref': 'provider://remote-https',
        'agent_profile': 'oasis7_p0_low_freq_npc',
        'fixture_id': 'provider_bridge_live_gate',
        'timeout_budget_ms': 15000,
    }
    with tempfile.NamedTemporaryFile('w', delete=False) as handle:
        json.dump(payload, handle)
        request_path = handle.name
    result = subprocess.run([
        'curl', '-sS', '--max-time', timeout, '-X', 'POST',
        'http://127.0.0.1:5841/v1/world-simulator/decision',
        '-H', 'Authorization: Bearer ' + auth,
        '-H', 'Content-Type: application/json',
        '--data-binary', '@' + request_path,
    ], text=True, capture_output=True)
    if result.returncode != 0:
        errors.append('curl:%s:%s' % (result.returncode, (result.stderr.strip() or 'curl failed')))
        break
    decoded = json.loads(result.stdout)
    error = decoded.get('provider_error')
    diagnostics = decoded.get('diagnostics') or {}
    if error is None:
        successes += 1
    else:
        errors.append(str(error.get('code') or ''))
        error_messages.append(str(error.get('message') or '')[:500])
    if diagnostics.get('provider_version'):
        versions.append(diagnostics.get('provider_version'))
matched_expected_error = bool(expected_error_substr) and any(
    expected_error_substr.lower() in text.lower()
    for text in errors + error_messages
)
status = 'pass' if successes >= min_successes and (not expected_error_substr or matched_expected_error) else 'fail'
print(json.dumps({
    'status': status,
    'decision_count': count,
    'decision_successes': successes,
    'provider_versions': sorted(set(versions)),
    'provider_error_codes': errors,
    'provider_error_messages': error_messages,
}, sort_keys=True))
if status != 'pass':
    raise SystemExit(1)
PY"
  echo "== ${label} =="
  ssh_capture "$host" "$remote_command"
}

test_selector=""
prod_selector=""
if [[ "$RUN_LOOPBACK" == "1" && ( "$LOOPBACK_TARGET" == "204" || "$LOOPBACK_TARGET" == "both" ) || "$LOWQUOTA_TARGET" == "204" ]]; then
  test_selector="$(selector_for_host "$TEST_HOST")"
fi
if [[ "$RUN_PUBLIC" == "1" || "$RUN_LOOPBACK" == "1" && ( "$LOOPBACK_TARGET" == "205" || "$LOOPBACK_TARGET" == "both" ) || "$LOWQUOTA_TARGET" == "205" || "$LOWQUOTA_TARGET" == "public205" ]]; then
  prod_selector="$(selector_for_host "$PROD_HOST")"
fi

if [[ "$RUN_LOOPBACK" == "1" ]]; then
  if [[ "$LOOPBACK_TARGET" == "204" || "$LOOPBACK_TARGET" == "both" ]]; then
    run_loopback_host_smoke "$TEST_HOST" "$test_selector" "204 loopback provider decision" "$DECISION_COUNT" "$DECISION_COUNT"
  fi
  if [[ "$LOOPBACK_TARGET" == "205" || "$LOOPBACK_TARGET" == "both" ]]; then
    run_loopback_host_smoke "$PROD_HOST" "$prod_selector" "205 loopback provider decision" "$DECISION_COUNT" "$DECISION_COUNT"
  fi
fi

if [[ "$RUN_PUBLIC" == "1" ]]; then
  run_contract_smoke "205 public nginx provider decision" "$PUBLIC_BASE_URL" "$prod_selector" "$DECISION_COUNT" "$DECISION_COUNT"
fi

case "$LOWQUOTA_TARGET" in
  "")
    ;;
  204)
    run_loopback_host_smoke "$TEST_HOST" "$test_selector" "204 lowquota loopback provider decision" "$LOWQUOTA_DECISION_COUNT" 1 "$LOWQUOTA_EXPECT_SUBSTR"
    ;;
  205)
    run_loopback_host_smoke "$PROD_HOST" "$prod_selector" "205 lowquota loopback provider decision" "$LOWQUOTA_DECISION_COUNT" 1 "$LOWQUOTA_EXPECT_SUBSTR"
    ;;
  public205)
    run_contract_smoke "205 public lowquota provider decision" "$PUBLIC_BASE_URL" "$prod_selector" "$LOWQUOTA_DECISION_COUNT" 1 --expect-provider-error-code-substr "$LOWQUOTA_EXPECT_SUBSTR"
    ;;
  *)
    echo "error: --lowquota-target must be 204, 205, or public205" >&2
    exit 2
    ;;
esac

if [[ "$RUN_ACCOUNTING" == "1" ]]; then
  [[ "$ACCOUNTING_ALLOW_MUTATION" == "1" ]] || { echo "error: accounting mode mutates live state; pass --i-understand-accounting-mutates-live-state" >&2; exit 2; }
  [[ -f "$ACCOUNTING_BIND_PAYLOAD_FILE" ]] || { echo "error: --accounting-bind-payload-file is required" >&2; exit 2; }
  [[ -n "$ACCOUNTING_TRANSFER_COMMAND" ]] || { echo "error: --accounting-transfer-command is required" >&2; exit 2; }
  bind_payload="$(python3 - "$ACCOUNTING_BIND_PAYLOAD_FILE" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding='utf-8'))
print(json.dumps(payload, separators=(',', ':')))
PY
)"
  accounting_result="$(ssh_capture "$ACCOUNTING_HOST" "BIND_PAYLOAD=$(printf '%q' "$bind_payload") PRICING_VERSION=$(printf '%q' "$ACCOUNTING_PRICING_VERSION") python3 - <<'PY'
import json, os, subprocess
bind_payload = os.environ['BIND_PAYLOAD']
pricing_version = os.environ['PRICING_VERSION']
def curl_json(args, body=None):
    cmd = ['curl', '-sS', '--max-time', '15'] + args
    if body is not None:
        cmd += ['-H', 'Content-Type: application/json', '-d', body]
    result = subprocess.run(cmd, text=True, capture_output=True)
    if result.returncode != 0:
        raise SystemExit(result.stderr.strip() or 'curl failed')
    return json.loads(result.stdout)
bind = curl_json(['-X', 'POST', 'http://127.0.0.1:5852/v1/bridge/bind'], bind_payload)
bridge_user_id = bind.get('bridge_user_id')
if not bridge_user_id:
    raise SystemExit('bind did not return bridge_user_id')
route = curl_json(['-X', 'POST', 'http://127.0.0.1:5852/v1/bridge/deposit-route'], json.dumps({
    'bridge_user_id': bridge_user_id,
    'pricing_version': pricing_version,
    'topup_plan_id': None,
}))
print(json.dumps({
    'bridge_user_id': bridge_user_id,
    'route_id': route.get('route_id'),
    'deposit_account_id': route.get('deposit_account_id'),
    'route_status': route.get('route_status'),
}, sort_keys=True))
PY")"
  echo "== accounting bind/deposit-route =="
  echo "$accounting_result"
  TEST_BRIDGE_USER_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["bridge_user_id"])' <<<"$accounting_result")"
  TEST_DEPOSIT_ROUTE_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["route_id"])' <<<"$accounting_result")"
  TEST_DEPOSIT_ACCOUNT_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["deposit_account_id"])' <<<"$accounting_result")"
  TEST_BRIDGE_BASE_URL="http://${ACCOUNTING_HOST}:5852"
  TEST_PROVIDER_BASE_URL="http://${ACCOUNTING_HOST}:5841"
  export TEST_BRIDGE_USER_ID TEST_DEPOSIT_ROUTE_ID TEST_DEPOSIT_ACCOUNT_ID TEST_BRIDGE_BASE_URL TEST_PROVIDER_BASE_URL
  bash -lc "$ACCOUNTING_TRANSFER_COMMAND"
  echo "== accounting reconcile =="
  ssh_capture "$ACCOUNTING_HOST" "curl -sS --max-time 15 -X POST http://127.0.0.1:5852/v1/bridge/reconcile"
fi

echo "provider bridge live gate passed"
