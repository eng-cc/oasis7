#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-preflight.sh \
    --bundle <path> \
    --sequencer-status-url <url> \
    --sequencer-ip <ip> \
    --sequencer-port <port> \
    --storage-status-url <url> \
    --storage-ip <ip> \
    --storage-port <port> \
    [--sequencer-ssh-host <user@host>] \
    [--sequencer-sshpass-env <env-name>] \
    [--storage-ssh-host <user@host>] \
    [--storage-sshpass-env <env-name>] \
    [--observer-env <path> ...] \
    [--seed-root <path> ...] \
    [--out-dir <path>]

  ./scripts/p2p-public-testnet-preflight.sh \
    --bundle <path> \
    --sequencer-status-json <path> \
    --sequencer-ip <ip> \
    --sequencer-port <port> \
    --storage-status-json <path> \
    --storage-ip <ip> \
    --storage-port <port> \
    [--observer-env <path> ...] \
    [--seed-root <path> ...] \
    [--out-dir <path>]

Description:
  Orchestrate the governed public_testnet preflight:
    1. capture deployment truth
    2. refresh observer bootstrap peer ids from live validator peer ids
    3. verify optional seed/state-sync closure roots

  Outputs:
    <out-dir>/deployment-truth.json
    <out-dir>/seed-closure-<name>.json (per --seed-root)
    <out-dir>/preflight-summary.json
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_command() {
  local name=$1
  command -v "$name" >/dev/null 2>&1 || die "missing command: $name"
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

BUNDLE_PATH=""
SEQUENCER_STATUS_URL=""
SEQUENCER_STATUS_JSON=""
SEQUENCER_IP=""
SEQUENCER_PORT=""
STORAGE_STATUS_URL=""
STORAGE_STATUS_JSON=""
STORAGE_IP=""
STORAGE_PORT=""
SEQUENCER_SSH_HOST=""
SEQUENCER_SSHPASS_ENV=""
STORAGE_SSH_HOST=""
STORAGE_SSHPASS_ENV=""
OUT_DIR=""
OBSERVER_ENVS=()
SEED_ROOTS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bundle)
      BUNDLE_PATH=${2:-}
      shift 2
      ;;
    --sequencer-status-url)
      SEQUENCER_STATUS_URL=${2:-}
      shift 2
      ;;
    --sequencer-status-json)
      SEQUENCER_STATUS_JSON=${2:-}
      shift 2
      ;;
    --sequencer-ip)
      SEQUENCER_IP=${2:-}
      shift 2
      ;;
    --sequencer-port)
      SEQUENCER_PORT=${2:-}
      shift 2
      ;;
    --storage-status-url)
      STORAGE_STATUS_URL=${2:-}
      shift 2
      ;;
    --storage-status-json)
      STORAGE_STATUS_JSON=${2:-}
      shift 2
      ;;
    --storage-ip)
      STORAGE_IP=${2:-}
      shift 2
      ;;
    --storage-port)
      STORAGE_PORT=${2:-}
      shift 2
      ;;
    --sequencer-ssh-host)
      SEQUENCER_SSH_HOST=${2:-}
      shift 2
      ;;
    --sequencer-sshpass-env)
      SEQUENCER_SSHPASS_ENV=${2:-}
      shift 2
      ;;
    --storage-ssh-host)
      STORAGE_SSH_HOST=${2:-}
      shift 2
      ;;
    --storage-sshpass-env)
      STORAGE_SSHPASS_ENV=${2:-}
      shift 2
      ;;
    --observer-env)
      OBSERVER_ENVS+=("${2:-}")
      shift 2
      ;;
    --seed-root)
      SEED_ROOTS+=("${2:-}")
      shift 2
      ;;
    --out-dir)
      OUT_DIR=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

require_command jq

[[ -n "$BUNDLE_PATH" ]] || die "--bundle is required"
[[ -n "$SEQUENCER_IP" && -n "$SEQUENCER_PORT" ]] || die "--sequencer-ip and --sequencer-port are required"
[[ -n "$STORAGE_IP" && -n "$STORAGE_PORT" ]] || die "--storage-ip and --storage-port are required"

source_count=0
[[ -n "$SEQUENCER_STATUS_URL" ]] && source_count=$((source_count + 1))
[[ -n "$SEQUENCER_STATUS_JSON" ]] && source_count=$((source_count + 1))
[[ "$source_count" -eq 1 ]] || die "provide exactly one of --sequencer-status-url or --sequencer-status-json"

source_count=0
[[ -n "$STORAGE_STATUS_URL" ]] && source_count=$((source_count + 1))
[[ -n "$STORAGE_STATUS_JSON" ]] && source_count=$((source_count + 1))
[[ "$source_count" -eq 1 ]] || die "provide exactly one of --storage-status-url or --storage-status-json"

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$repo_root/.tmp/public-testnet-preflight-$(date +%Y%m%d-%H%M%S)"
fi
mkdir -p "$OUT_DIR"

truth_json="$OUT_DIR/deployment-truth.json"

capture_args=(
  --bundle "$BUNDLE_PATH"
)
if [[ -n "$SEQUENCER_STATUS_URL" ]]; then
  capture_args+=(--sequencer-status-url "$SEQUENCER_STATUS_URL")
else
  capture_args+=(--sequencer-status-json "$SEQUENCER_STATUS_JSON")
fi
if [[ -n "$STORAGE_STATUS_URL" ]]; then
  capture_args+=(--storage-status-url "$STORAGE_STATUS_URL")
else
  capture_args+=(--storage-status-json "$STORAGE_STATUS_JSON")
fi
if [[ -n "$SEQUENCER_SSH_HOST" ]]; then
  capture_args+=(--sequencer-ssh-host "$SEQUENCER_SSH_HOST")
fi
if [[ -n "$SEQUENCER_SSHPASS_ENV" ]]; then
  capture_args+=(--sequencer-sshpass-env "$SEQUENCER_SSHPASS_ENV")
fi
if [[ -n "$STORAGE_SSH_HOST" ]]; then
  capture_args+=(--storage-ssh-host "$STORAGE_SSH_HOST")
fi
if [[ -n "$STORAGE_SSHPASS_ENV" ]]; then
  capture_args+=(--storage-sshpass-env "$STORAGE_SSHPASS_ENV")
fi
capture_args+=(--out "$truth_json")

./scripts/p2p-public-testnet-capture-truth.sh "${capture_args[@]}"

if [[ ${#OBSERVER_ENVS[@]} -gt 0 ]]; then
  refresh_args=(
    --sequencer-ip "$SEQUENCER_IP"
    --sequencer-port "$SEQUENCER_PORT"
    --storage-ip "$STORAGE_IP"
    --storage-port "$STORAGE_PORT"
  )
  if [[ -n "$SEQUENCER_STATUS_URL" ]]; then
    refresh_args+=(--sequencer-status-url "$SEQUENCER_STATUS_URL")
  else
    refresh_args+=(--sequencer-status-json "$SEQUENCER_STATUS_JSON")
  fi
  if [[ -n "$STORAGE_STATUS_URL" ]]; then
    refresh_args+=(--storage-status-url "$STORAGE_STATUS_URL")
  else
    refresh_args+=(--storage-status-json "$STORAGE_STATUS_JSON")
  fi
  for env_file in "${OBSERVER_ENVS[@]}"; do
    refresh_args+=(--env-file "$env_file")
  done
  bootstrap_csv=$(./scripts/p2p-public-testnet-refresh-bootstrap-peers.sh "${refresh_args[@]}")
else
  bootstrap_csv=""
fi

seed_reports=()
if (( ${#SEED_ROOTS[@]} > 0 )); then
  for seed_root in "${SEED_ROOTS[@]}"; do
    base_name=$(basename "$seed_root")
    report_path="$OUT_DIR/seed-closure-${base_name}.json"
    ./scripts/p2p-verify-state-sync-closure.sh \
      --world-dir "$seed_root/world" \
      --execution-records-dir "$seed_root/execution-records" \
      --store-dir "$seed_root/store" \
      --out "$report_path"
    seed_reports+=("$report_path")
  done
fi

seed_reports_json="[]"
if [[ ${#seed_reports[@]} -gt 0 ]]; then
  seed_reports_json=$(printf '%s\n' "${seed_reports[@]}" | jq -R . | jq -s .)
fi

jq -n \
  --arg out_dir "$OUT_DIR" \
  --arg deployment_truth "$truth_json" \
  --arg bootstrap_csv "$bootstrap_csv" \
  --argjson seed_reports "$seed_reports_json" \
  '{
    ok: true,
    out_dir: $out_dir,
    deployment_truth_path: $deployment_truth,
    refreshed_bootstrap_peers_csv: (if $bootstrap_csv == "" then null else $bootstrap_csv end),
    seed_closure_reports: $seed_reports
  }' >"$OUT_DIR/preflight-summary.json"

cat <<EOF
deployment_truth_path=$truth_json
preflight_summary_path=$OUT_DIR/preflight-summary.json
EOF
if [[ -n "$bootstrap_csv" ]]; then
  printf 'refreshed_bootstrap_peers_csv=%s\n' "$bootstrap_csv"
fi
if [[ ${#seed_reports[@]} -gt 0 ]]; then
  printf 'seed_closure_reports=%s\n' "${seed_reports[*]}"
fi
