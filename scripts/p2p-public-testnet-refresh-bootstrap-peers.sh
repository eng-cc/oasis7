#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-refresh-bootstrap-peers.sh \
    --sequencer-status-url <url> \
    --sequencer-ip <ip> \
    --sequencer-port <port> \
    --storage-status-url <url> \
    --storage-ip <ip> \
    --storage-port <port> \
    --env-file <path> \
    [--env-file <path> ...]

  ./scripts/p2p-public-testnet-refresh-bootstrap-peers.sh \
    --sequencer-status-json <path> \
    --sequencer-ip <ip> \
    --sequencer-port <port> \
    --storage-status-json <path> \
    --storage-ip <ip> \
    --storage-port <port> \
    --env-file <path> \
    [--env-file <path> ...]

Description:
  Refresh local observer env files so REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV
  points at the live validator libp2p peer ids instead of stale historical ids.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_file() {
  local path=$1
  [[ -f "$path" ]] || die "missing file: $path"
}

SEQUENCER_STATUS_URL=""
SEQUENCER_STATUS_JSON=""
SEQUENCER_IP=""
SEQUENCER_PORT=""
STORAGE_STATUS_URL=""
STORAGE_STATUS_JSON=""
STORAGE_IP=""
STORAGE_PORT=""
ENV_FILES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
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
    --env-file)
      ENV_FILES+=("${2:-}")
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

require_command() {
  local name=$1
  command -v "$name" >/dev/null 2>&1 || die "missing command: $name"
}

require_command jq
require_command curl

[[ ${#ENV_FILES[@]} -gt 0 ]] || die "at least one --env-file is required"
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

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-refresh-bootstrap.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT

sequencer_status_path="$tmp_dir/sequencer-status.json"
storage_status_path="$tmp_dir/storage-status.json"

if [[ -n "$SEQUENCER_STATUS_URL" ]]; then
  curl -fsSL "$SEQUENCER_STATUS_URL" -o "$sequencer_status_path"
else
  cp "$SEQUENCER_STATUS_JSON" "$sequencer_status_path"
fi

if [[ -n "$STORAGE_STATUS_URL" ]]; then
  curl -fsSL "$STORAGE_STATUS_URL" -o "$storage_status_path"
else
  cp "$STORAGE_STATUS_JSON" "$storage_status_path"
fi

sequencer_peer_id=$(jq -r '.replication.local_peer_id // empty' "$sequencer_status_path")
storage_peer_id=$(jq -r '.replication.local_peer_id // empty' "$storage_status_path")
[[ -n "$sequencer_peer_id" ]] || die "sequencer status missing replication.local_peer_id"
[[ -n "$storage_peer_id" ]] || die "storage status missing replication.local_peer_id"

bootstrap_csv="/ip4/${SEQUENCER_IP}/tcp/${SEQUENCER_PORT}/p2p/${sequencer_peer_id},/ip4/${STORAGE_IP}/tcp/${STORAGE_PORT}/p2p/${storage_peer_id}"

python3 - "$bootstrap_csv" "${ENV_FILES[@]}" <<'PY'
import pathlib
import sys

bootstrap_csv = sys.argv[1]
for raw_path in sys.argv[2:]:
    path = pathlib.Path(raw_path)
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    replaced = False
    out_lines = []
    for line in lines:
        if line.startswith("REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV="):
            out_lines.append(f"REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV={bootstrap_csv}")
            replaced = True
        else:
            out_lines.append(line)
    if not replaced:
        raise SystemExit(f"env file missing REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV: {path}")
    path.write_text("\n".join(out_lines) + "\n", encoding="utf-8")
PY

printf '%s\n' "$bootstrap_csv"
