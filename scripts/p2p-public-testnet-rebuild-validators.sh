#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-rebuild-validators.sh \
    --config-dir <path> \
    --world-dir <path> \
    --sequencer-ssh-host <user@host> \
    --sequencer-sshpass-env <env-name> \
    --sequencer-service <name> \
    --sequencer-status-url <url> \
    --storage-ssh-host <user@host> \
    --storage-sshpass-env <env-name> \
    --storage-service <name> \
    --storage-status-url <url> \
    [--stack-root <path>] \
    [--out-dir <path>] \
    [--poll-attempts <n>] \
    [--poll-sleep-seconds <n>]

Description:
  Stage deployment-truth config/world onto the validator pair, destructively
  reset old validator chain state, restart the validator services in order,
  and capture live status evidence after restart.

  This script assumes the validator hosts already have the intended runtime
  package installed. It rebuilds chain state from the provided config/world
  truth and preserves protected host assets such as config/node-keypair.toml.
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

require_dir() {
  local path=$1
  [[ -d "$path" ]] || die "missing directory: $path"
}

require_file() {
  local path=$1
  [[ -f "$path" ]] || die "missing file: $path"
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

CONFIG_DIR=""
WORLD_DIR=""
SEQUENCER_SSH_HOST=""
SEQUENCER_SSHPASS_ENV=""
SEQUENCER_SERVICE=""
SEQUENCER_STATUS_URL=""
STORAGE_SSH_HOST=""
STORAGE_SSHPASS_ENV=""
STORAGE_SERVICE=""
STORAGE_STATUS_URL=""
STACK_ROOT="/opt/oasis7/p2p-testnet"
OUT_DIR=""
POLL_ATTEMPTS=20
POLL_SLEEP_SECONDS=3

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-dir)
      CONFIG_DIR=${2:-}
      shift 2
      ;;
    --world-dir)
      WORLD_DIR=${2:-}
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
    --sequencer-service)
      SEQUENCER_SERVICE=${2:-}
      shift 2
      ;;
    --sequencer-status-url)
      SEQUENCER_STATUS_URL=${2:-}
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
    --storage-service)
      STORAGE_SERVICE=${2:-}
      shift 2
      ;;
    --storage-status-url)
      STORAGE_STATUS_URL=${2:-}
      shift 2
      ;;
    --stack-root)
      STACK_ROOT=${2:-}
      shift 2
      ;;
    --out-dir)
      OUT_DIR=${2:-}
      shift 2
      ;;
    --poll-attempts)
      POLL_ATTEMPTS=${2:-}
      shift 2
      ;;
    --poll-sleep-seconds)
      POLL_SLEEP_SECONDS=${2:-}
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
require_command curl
require_command ssh
require_command sshpass
require_command tar
require_command mktemp
require_command shasum

[[ -n "$CONFIG_DIR" ]] || die "--config-dir is required"
[[ -n "$WORLD_DIR" ]] || die "--world-dir is required"
[[ -n "$SEQUENCER_SSH_HOST" ]] || die "--sequencer-ssh-host is required"
[[ -n "$SEQUENCER_SSHPASS_ENV" ]] || die "--sequencer-sshpass-env is required"
[[ -n "$SEQUENCER_SERVICE" ]] || die "--sequencer-service is required"
[[ -n "$SEQUENCER_STATUS_URL" ]] || die "--sequencer-status-url is required"
[[ -n "$STORAGE_SSH_HOST" ]] || die "--storage-ssh-host is required"
[[ -n "$STORAGE_SSHPASS_ENV" ]] || die "--storage-sshpass-env is required"
[[ -n "$STORAGE_SERVICE" ]] || die "--storage-service is required"
[[ -n "$STORAGE_STATUS_URL" ]] || die "--storage-status-url is required"

require_dir "$CONFIG_DIR"
require_dir "$WORLD_DIR"

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$repo_root/.tmp/public-testnet-validator-rebuild-$(date +%Y%m%d-%H%M%S)"
fi
mkdir -p "$OUT_DIR"
CONTROL_DIR="$(mktemp -d "/tmp/o7pt-ssh.XXXXXX")"
cleanup() {
  if [[ -n "${SEQUENCER_CONTROL_PATH:-}" && -S "${SEQUENCER_CONTROL_PATH:-}" ]]; then
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -S "$SEQUENCER_CONTROL_PATH" -O exit "$SEQUENCER_SSH_HOST" >/dev/null 2>&1 || true
  fi
  if [[ -n "${STORAGE_CONTROL_PATH:-}" && -S "${STORAGE_CONTROL_PATH:-}" ]]; then
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -S "$STORAGE_CONTROL_PATH" -O exit "$STORAGE_SSH_HOST" >/dev/null 2>&1 || true
  fi
  rm -rf "$CONTROL_DIR"
}
trap cleanup EXIT

config_files=()
while IFS= read -r path; do
  config_files+=("$path")
done < <(find "$CONFIG_DIR" -maxdepth 1 -type f | sort)
[[ ${#config_files[@]} -gt 0 ]] || die "no top-level config files found in $CONFIG_DIR"

control_path_for() {
  local host=$1
  local label
  label=$(printf '%s' "$host" | shasum | awk '{print substr($1, 1, 10)}')
  printf '%s/%s.sock\n' "$CONTROL_DIR" "$label"
}

open_master_connection() {
  local host=$1
  local sshpass_env_name=$2
  local control_path
  control_path=$(control_path_for "$host")
  if [[ -S "$control_path" ]]; then
    printf '%s\n' "$control_path"
    return 0
  fi
  local sshpass_value=${!sshpass_env_name:-}
  [[ -n "$sshpass_value" ]] || die "ssh password env is empty: $sshpass_env_name"
  SSHPASS="$sshpass_value" sshpass -e ssh \
    -M -N -f \
    -o ControlMaster=yes \
    -o ControlPersist=600 \
    -S "$control_path" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "$host"
  printf '%s\n' "$control_path"
}

SEQUENCER_CONTROL_PATH=$(open_master_connection "$SEQUENCER_SSH_HOST" "$SEQUENCER_SSHPASS_ENV")
STORAGE_CONTROL_PATH=$(open_master_connection "$STORAGE_SSH_HOST" "$STORAGE_SSHPASS_ENV")

ssh_run() {
  local host=$1
  local control_path=$2
  shift 2
  ssh \
    -S "$control_path" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "$host" \
    "$@"
}

json_sequencer_ok() {
  local path=$1
  jq -e '
    .running == true
    and (.last_error == null or .last_error == "null")
    and (.readiness.status // null) == "ready"
    and (.consensus.storage_challenge_network_degraded_height // null) == null
    and ((.observability.storage_challenge_network_degraded // false) | not)
    and ((.consensus.committed_height // 0) > 0)
    and ((.consensus.last_execution_height // 0) > 0)
  ' "$path" >/dev/null 2>&1
}

json_liveness_ok() {
  local path=$1
  jq -e '
    .running == true
    and (.last_error == null or .last_error == "null")
  ' "$path" >/dev/null 2>&1
}

json_storage_ok() {
  local path=$1
  jq -e '
    .running == true
    and (.last_error == null or .last_error == "null")
    and (.readiness.status // null) == "ready"
    and (.consensus.storage_challenge_network_degraded_height // null) == null
    and ((.observability.storage_challenge_network_degraded // false) | not)
    and ((.consensus.committed_height // 0) > 0)
    and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
    and (.replication.connected_peers | length) >= 1
  ' "$path" >/dev/null 2>&1
}

poll_status_with_check() {
  local url=$1
  local out_path=$2
  local label=$3
  local check_fn=$4
  local attempt=1
  while (( attempt <= POLL_ATTEMPTS )); do
    if curl -fsSL "$url" -o "$out_path.tmp"; then
      if "$check_fn" "$out_path.tmp"; then
        mv "$out_path.tmp" "$out_path"
        return 0
      fi
    fi
    attempt=$((attempt + 1))
    sleep "$POLL_SLEEP_SECONDS"
  done
  if [[ -f "$out_path.tmp" ]]; then
    mv "$out_path.tmp" "$out_path"
  fi
  return 1
}

stage_host() {
  local host=$1
  local control_path=$2
  ssh_run "$host" "$control_path" \
    "mkdir -p '$STACK_ROOT/config/doc/testing/evidence' '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world'"

  local file
  for file in "${config_files[@]}"; do
    require_file "$file"
    local base
    base=$(basename "$file")
    ssh_run "$host" "$control_path" "cat > '$STACK_ROOT/config/$base'" <"$file"
    ssh_run "$host" "$control_path" "cp '$STACK_ROOT/config/$base' '$STACK_ROOT/config/doc/testing/evidence/$base'"
  done

  ssh_run "$host" "$control_path" \
    "rm -rf '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world' && mkdir -p '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world'"
  tar -C "$WORLD_DIR" -cf - . \
    | ssh_run "$host" "$control_path" "tar -C '$STACK_ROOT/staged-world' -xf -"
  ssh_run "$host" "$control_path" \
    "cp -R '$STACK_ROOT/staged-world/.' '$STACK_ROOT/data/execution-world/'"

  sync_staged_deployment_truth "$host" "$control_path" >&2
}

sync_staged_deployment_truth() {
  local host=$1
  local control_path=$2
  ssh_run "$host" "$control_path" \
    "STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

stack_root = Path(os.environ['STACK_ROOT'])
config_dir = stack_root / 'config'
env_path = config_dir / 'node.env'
runtime_path = stack_root / 'current' / 'bin' / 'oasis7_chain_runtime'

if not env_path.is_file():
    raise SystemExit(f'missing node env: {env_path}')
if not runtime_path.is_file():
    raise SystemExit(f'missing installed runtime: {runtime_path}')

env_values = {}
env_values['STACK_ROOT'] = str(stack_root)
for raw in env_path.read_text(encoding='utf-8').splitlines():
    stripped = raw.strip()
    if not stripped or stripped.startswith('#') or '=' not in stripped:
        continue
    key, value = stripped.split('=', 1)
    if (
        len(value) >= 2
        and value[0] == value[-1]
        and value[0] in (chr(34), chr(39))
    ):
        value = value[1:-1]
    env_values[key] = value

def expand_env_value(value: str) -> str:
    previous = value
    for _ in range(8):
        expanded = previous
        for name, replacement in sorted(env_values.items(), key=lambda item: len(item[0]), reverse=True):
            expanded = expanded.replace(chr(36) + '{' + name + '}', replacement)
            expanded = expanded.replace(chr(36) + name, replacement)
        if chr(36) in expanded:
            raise SystemExit(f'unsupported variable in GENESIS_VALIDATOR_REGISTRY_PATH: {expanded}')
        if expanded == previous:
            return expanded
        previous = expanded
    raise SystemExit('GENESIS_VALIDATOR_REGISTRY_PATH variable expansion did not converge')

registry_raw = env_values.get('GENESIS_VALIDATOR_REGISTRY_PATH')
if not registry_raw:
    raise SystemExit(f'missing GENESIS_VALIDATOR_REGISTRY_PATH in {env_path}')
registry_path = Path(expand_env_value(registry_raw))
if not registry_path.is_absolute():
    registry_path = stack_root / registry_path
try:
    registry_path.resolve(strict=False).relative_to(stack_root.resolve(strict=False))
except ValueError:
    raise SystemExit(f'GENESIS_VALIDATOR_REGISTRY_PATH must stay under stack root: {registry_path}')
if not registry_path.is_file():
    raise SystemExit(f'GENESIS_VALIDATOR_REGISTRY_PATH does not exist after staging: {registry_path}')

registry_data = json.loads(registry_path.read_text(encoding='utf-8'))
validators = registry_data.get('validators')
if not isinstance(validators, list) or not validators:
    raise SystemExit(f'GENESIS_VALIDATOR_REGISTRY_PATH has no validators: {registry_path}')

signer_pairs = []
for validator in validators:
    node_id = validator.get('node_id')
    public_key = validator.get('finality_signer_public_key')
    if not node_id or not public_key:
        raise SystemExit(f'validator registry entry is missing node_id/finality_signer_public_key in {registry_path}')
    signer_pairs.append(f'{node_id}:{public_key}')
signers_csv = ','.join(signer_pairs)

lines = env_path.read_text(encoding='utf-8').splitlines()
rewrote_signers = False
rendered = []
for line in lines:
    if line.startswith('NODE_VALIDATOR_SIGNERS_CSV='):
        rendered.append(f'NODE_VALIDATOR_SIGNERS_CSV={signers_csv}')
        rewrote_signers = True
    else:
        rendered.append(line)
if not rewrote_signers:
    rendered.append(f'NODE_VALIDATOR_SIGNERS_CSV={signers_csv}')
env_path.write_text('\\n'.join(rendered) + '\\n', encoding='utf-8')

digest = hashlib.sha256()
with runtime_path.open('rb') as fh:
    for chunk in iter(lambda: fh.read(1024 * 1024), b''):
        digest.update(chunk)
runtime_sha = digest.hexdigest()
runtime_size = runtime_path.stat().st_size

buildinfo = {}
buildinfo_path = stack_root / 'DEPLOYED_BUILDINFO'
if buildinfo_path.is_file():
    for raw in buildinfo_path.read_text(encoding='utf-8').splitlines():
        key, sep, value = raw.partition('=')
        if sep:
            buildinfo[key] = value

bundle_paths = sorted(config_dir.rglob('public-testnet-governed-bootstrap-bundle-2026-06-06.json'))
if not bundle_paths:
    raise SystemExit(f'no governed bootstrap bundle found under {config_dir}')

updated_by = 'p2p-public-testnet-rebuild-validators staged deployment truth sync'
if buildinfo.get('package_version') or buildinfo.get('run_id'):
    updated_by += f\" package={buildinfo.get('package_version', 'unknown')} run={buildinfo.get('run_id', 'unknown')}\"

for bundle_path in bundle_paths:
    data = json.loads(bundle_path.read_text(encoding='utf-8'))
    runtime = data.setdefault('runtime_build', {})
    runtime['kind'] = 'file'
    runtime['path'] = str(runtime_path)
    runtime['resolved_path'] = str(runtime_path)
    runtime['sha256'] = runtime_sha
    runtime['size_bytes'] = runtime_size
    runtime['updated_by'] = updated_by
    if buildinfo.get('commit'):
        runtime['git_commit'] = buildinfo['commit']
        data['git_commit'] = buildinfo['commit']
    if buildinfo.get('package_version'):
        runtime['package_version'] = buildinfo['package_version']
    if buildinfo.get('run_id'):
        runtime['run_id'] = buildinfo['run_id']
    data['updated_by'] = updated_by
    bundle_path.write_text(
        json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + '\\n',
        encoding='utf-8',
    )

print(f'synced_validator_signer_count={len(signer_pairs)}')
print(f'synced_runtime_sha256={runtime_sha}')
print(f'synced_bundle_count={len(bundle_paths)}')
PY"
}

cleanup_host_processes() {
  local host=$1
  local control_path=$2
  local service=$3
  ssh_run "$host" "$control_path" \
    "systemctl stop '$service' || true; systemctl reset-failed '$service' || true; STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
import os
import signal
import subprocess
import time

stack_root = os.environ['STACK_ROOT']
needles = (
    f'{stack_root}/current/bin/oasis7_chain_runtime',
    f'{stack_root}/bin/start-node.sh',
)

def matching_pids():
    current = os.getpid()
    parent = os.getppid()
    out = subprocess.run(['ps', '-eo', 'pid=,args='], text=True, stdout=subprocess.PIPE, check=False).stdout
    pids = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        raw_pid, _, args = line.partition(' ')
        try:
            pid = int(raw_pid)
        except ValueError:
            continue
        if pid in (current, parent):
            continue
        if any(needle in args for needle in needles):
            pids.append(pid)
    return pids

deadline = time.monotonic() + 10
while time.monotonic() < deadline:
    if not matching_pids():
        break
    time.sleep(0.25)

for sig in (signal.SIGTERM, signal.SIGKILL):
    pids = matching_pids()
    if not pids:
        break
    for pid in pids:
        try:
            os.kill(pid, sig)
        except ProcessLookupError:
            pass
    time.sleep(1)
PY
"
}

reset_host() {
  local host=$1
  local control_path=$2
  local service=$3
  cleanup_host_processes "$host" "$control_path" "$service"
  ssh_run "$host" "$control_path" \
    "rm -rf '$STACK_ROOT/data/execution-records' '$STACK_ROOT/data/storage' '$STACK_ROOT/data/runtime-root' '$STACK_ROOT/data/replication-root' '$STACK_ROOT/output/chain-runtime' '$STACK_ROOT/output/node-distfs'; mkdir -p '$STACK_ROOT/data/execution-records' '$STACK_ROOT/data/storage' '$STACK_ROOT/data/runtime-root' '$STACK_ROOT/data/replication-root' '$STACK_ROOT/output/chain-runtime' '$STACK_ROOT/output/node-distfs'"
}

start_host() {
  local host=$1
  local control_path=$2
  local service=$3
  ssh_run "$host" "$control_path" "systemctl reset-failed '$service' || true; systemctl start '$service'"
}

cleanup_after_failed_start() {
  local host=$1
  local control_path=$2
  local service=$3
  local label=$4
  local out_path=$5
  cleanup_host_processes "$host" "$control_path" "$service" || true
  die "$label failed checks after restart; see $out_path"
}

stage_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH"
stage_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH"

reset_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE"
reset_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH" "$STORAGE_SERVICE"

start_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE"
poll_status_with_check "$SEQUENCER_STATUS_URL" "$OUT_DIR/sequencer-liveness.json" "sequencer liveness" json_liveness_ok \
  || cleanup_after_failed_start "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE" "sequencer liveness" "$OUT_DIR/sequencer-liveness.json"

start_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH" "$STORAGE_SERVICE"
poll_status_with_check "$SEQUENCER_STATUS_URL" "$OUT_DIR/sequencer-status.json" "sequencer readiness" json_sequencer_ok \
  || cleanup_after_failed_start "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE" "sequencer readiness" "$OUT_DIR/sequencer-status.json"
poll_status_with_check "$STORAGE_STATUS_URL" "$OUT_DIR/storage-status.json" "storage readiness" json_storage_ok \
  || cleanup_after_failed_start "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH" "$STORAGE_SERVICE" "storage readiness" "$OUT_DIR/storage-status.json"

jq -n \
  --arg config_dir "$CONFIG_DIR" \
  --arg world_dir "$WORLD_DIR" \
  --arg stack_root "$STACK_ROOT" \
  --arg sequencer_status_url "$SEQUENCER_STATUS_URL" \
  --arg storage_status_url "$STORAGE_STATUS_URL" \
  --slurpfile sequencer "$OUT_DIR/sequencer-status.json" \
  --slurpfile storage "$OUT_DIR/storage-status.json" \
  '
    {
      config_dir: $config_dir,
      world_dir: $world_dir,
      stack_root: $stack_root,
      sequencer_status_url: $sequencer_status_url,
      storage_status_url: $storage_status_url,
      sequencer_status: $sequencer[0],
      storage_status: $storage[0]
    }
  ' >"$OUT_DIR/rebuild-summary.json"

jq -n \
  --slurpfile sequencer "$OUT_DIR/sequencer-status.json" \
  --slurpfile storage "$OUT_DIR/storage-status.json" \
  '{
    sequencer: {
      running: $sequencer[0].running,
      last_error: $sequencer[0].last_error,
      committed_height: $sequencer[0].consensus.committed_height,
      last_execution_height: $sequencer[0].consensus.last_execution_height,
      local_peer_id: $sequencer[0].replication.local_peer_id,
      connected_peers: $sequencer[0].replication.connected_peers
    },
    storage: {
      running: $storage[0].running,
      last_error: $storage[0].last_error,
      committed_height: $storage[0].consensus.committed_height,
      last_execution_height: $storage[0].consensus.last_execution_height,
      local_peer_id: $storage[0].replication.local_peer_id,
      connected_peers: $storage[0].replication.connected_peers
    }
  }'
