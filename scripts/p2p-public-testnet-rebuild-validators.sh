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
    [--poll-sleep-seconds <n>] \
    [--disable-ssh-multiplex]

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
DISABLE_SSH_MULTIPLEX=0

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
    --disable-ssh-multiplex)
      DISABLE_SSH_MULTIPLEX=1
      shift
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

evidence_files=()
if [[ -d "$CONFIG_DIR/doc/testing/evidence" ]]; then
  while IFS= read -r path; do
    evidence_files+=("$path")
  done < <(find "$CONFIG_DIR/doc/testing/evidence" -maxdepth 1 -type f | sort)
fi

network_manifest_path=""
for file in "${config_files[@]}"; do
  if jq -e '(.schema_version // "") == "oasis7.network_tier_manifest.v1"' "$file" >/dev/null 2>&1; then
    network_manifest_path=$file
    break
  fi
done
[[ -n "$network_manifest_path" ]] || die "missing oasis7.network_tier_manifest.v1 config"
WORLD_RESOURCE_WORLD_ID=$(jq -r '.network_id // empty' "$network_manifest_path")
WORLD_RESOURCE_CHAIN_ID=$(jq -r '.chain_id // .network_id // empty' "$network_manifest_path")
[[ -n "$WORLD_RESOURCE_WORLD_ID" ]] || die "network tier manifest missing network_id"
[[ -n "$WORLD_RESOURCE_CHAIN_ID" ]] || die "network tier manifest missing chain_id"

control_path_for() {
  local host=$1
  local label
  label=$(printf '%s' "$host" | shasum | awk '{print substr($1, 1, 10)}')
  printf '%s/%s.sock\n' "$CONTROL_DIR" "$label"
}

open_master_connection() {
  local host=$1
  local sshpass_env_name=$2
  if [[ "$DISABLE_SSH_MULTIPLEX" -eq 1 ]]; then
    printf '\n'
    return 0
  fi
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
  local ssh_args=()
  if [[ -n "$control_path" ]]; then
    ssh_args+=(-S "$control_path")
  else
    ssh_args+=(
      -o ControlMaster=no
      -o ControlPath=none
      -o PreferredAuthentications=password
      -o PubkeyAuthentication=no
      -o NumberOfPasswordPrompts=1
      -o ConnectTimeout=10
      -o ServerAliveInterval=15
      -o ServerAliveCountMax=2
    )
    if [[ "$host" == "$SEQUENCER_SSH_HOST" ]]; then
      local sequencer_sshpass=${!SEQUENCER_SSHPASS_ENV:-}
      [[ -n "$sequencer_sshpass" ]] || die "ssh password env is empty: $SEQUENCER_SSHPASS_ENV"
      SSHPASS="$sequencer_sshpass" sshpass -e ssh "${ssh_args[@]}" \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        "$host" \
        "$@"
      return $?
    fi
    if [[ "$host" == "$STORAGE_SSH_HOST" ]]; then
      local storage_sshpass=${!STORAGE_SSHPASS_ENV:-}
      [[ -n "$storage_sshpass" ]] || die "ssh password env is empty: $STORAGE_SSHPASS_ENV"
      SSHPASS="$storage_sshpass" sshpass -e ssh "${ssh_args[@]}" \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        "$host" \
        "$@"
      return $?
    fi
  fi
  ssh "${ssh_args[@]}" \
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
    and (((.readiness.failed_gates // []) | length) == 0)
    and (.consensus.storage_challenge_network_degraded_height // null) == null
    and ((.observability.storage_challenge_network_degraded // false) | not)
    and ((.consensus.committed_height // 0) > 0)
    and ((.consensus.last_execution_height // 0) > 0)
    and (((.consensus.last_execution_block_hash // "") | tostring | length) > 0)
    and (((.consensus.last_execution_state_root // "") | tostring | length) > 0)
    and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
    and ((.world_resource.readiness_status // null) == "ready")
    and (((.world_resource.failed_gates // []) | length) == 0)
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
    and (((.readiness.failed_gates // []) | length) == 0)
    and (.consensus.storage_challenge_network_degraded_height // null) == null
    and ((.observability.storage_challenge_network_degraded // false) | not)
    and (.replication.connected_peers | length) >= 1
    and ((.consensus.committed_height // 0) > 0)
    and ((.consensus.last_execution_height // 0) > 0)
    and (((.consensus.last_execution_block_hash // "") | tostring | length) > 0)
    and (((.consensus.last_execution_state_root // "") | tostring | length) > 0)
    and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
    and ((.world_resource.readiness_status // null) == "ready")
    and (((.world_resource.failed_gates // []) | length) == 0)
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
  if ((${#evidence_files[@]} > 0)); then
    for file in "${evidence_files[@]}"; do
      require_file "$file"
      local base
      base=$(basename "$file")
      ssh_run "$host" "$control_path" "cat > '$STACK_ROOT/config/doc/testing/evidence/$base'" <"$file"
    done
  fi

  ssh_run "$host" "$control_path" \
    "test -x '$STACK_ROOT/current/bin/oasis7_world_repair_rebuild' && '$STACK_ROOT/current/bin/oasis7_world_repair_rebuild' --help 2>&1 | grep -F -- '--generated-world-dir' >/dev/null"
  ssh_run "$host" "$control_path" \
    "rm -rf '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world' && mkdir -p '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world'"
  COPYFILE_DISABLE=1 tar -C "$WORLD_DIR" -cf - . \
    | ssh_run "$host" "$control_path" "tar -C '$STACK_ROOT/staged-world' -xf -"
  ssh_run "$host" "$control_path" \
    "find '$STACK_ROOT/staged-world' \\( -name '._*' -o -name '.DS_Store' \\) -delete"
  ssh_run "$host" "$control_path" \
    "'$STACK_ROOT/current/bin/oasis7_world_repair_rebuild' --generated-world-dir '$STACK_ROOT/staged-world' --output-world-dir '$STACK_ROOT/data/execution-world' --world-id '$WORLD_RESOURCE_WORLD_ID' --chain-id '$WORLD_RESOURCE_CHAIN_ID' --resource-commit-height 0 --resource-commit-hash genesis"
  ssh_run "$host" "$control_path" \
    "cp -R '$STACK_ROOT/staged-world/generated-scenario-world' '$STACK_ROOT/data/execution-world/generated-scenario-world' && cp '$STACK_ROOT/staged-world/world-generation-provenance.json' '$STACK_ROOT/data/execution-world/world-generation-provenance.json'"

  sync_staged_deployment_truth "$host" "$control_path" >&2
  import_staged_governance_registry "$host" "$control_path" >&2
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
generated_world_root = stack_root / 'data' / 'execution-world'
generated_world_sidecar_path = generated_world_root / 'generated-scenario-world'
world_generation_provenance_path = generated_world_root / 'world-generation-provenance.json'

if not env_path.is_file():
    raise SystemExit(f'missing node env: {env_path}')
if not runtime_path.is_file():
    raise SystemExit(f'missing installed runtime: {runtime_path}')
if not generated_world_sidecar_path.is_dir():
    raise SystemExit(f'missing staged generated world sidecar: {generated_world_sidecar_path}')
if not world_generation_provenance_path.is_file():
    raise SystemExit(f'missing staged world generation provenance: {world_generation_provenance_path}')

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
rewrote_adaptive_tick_scheduler = False
rendered = []
for line in lines:
    if line.startswith('NODE_VALIDATOR_SIGNERS_CSV='):
        rendered.append(f'NODE_VALIDATOR_SIGNERS_CSV={signers_csv}')
        rewrote_signers = True
    elif line.startswith('POS_ADAPTIVE_TICK_SCHEDULER='):
        rendered.append('POS_ADAPTIVE_TICK_SCHEDULER=1')
        rewrote_adaptive_tick_scheduler = True
    else:
        rendered.append(line)
if not rewrote_signers:
    rendered.append(f'NODE_VALIDATOR_SIGNERS_CSV={signers_csv}')
if not rewrote_adaptive_tick_scheduler:
    rendered.append('POS_ADAPTIVE_TICK_SCHEDULER=1')
env_path.write_text('\\n'.join(rendered) + '\\n', encoding='utf-8')

digest = hashlib.sha256()
with runtime_path.open('rb') as fh:
    for chunk in iter(lambda: fh.read(1024 * 1024), b''):
        digest.update(chunk)
runtime_sha = digest.hexdigest()
runtime_size = runtime_path.stat().st_size
provenance_digest = hashlib.sha256()
with world_generation_provenance_path.open('rb') as fh:
    for chunk in iter(lambda: fh.read(1024 * 1024), b''):
        provenance_digest.update(chunk)
world_generation_provenance_sha = provenance_digest.hexdigest()
world_generation_provenance_size = world_generation_provenance_path.stat().st_size

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

genesis_paths = sorted(config_dir.rglob('public-testnet-governed-bootstrap-genesis-2026-06-06.json'))
if not genesis_paths:
    raise SystemExit(f'no governed bootstrap genesis found under {config_dir}')

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
    if isinstance(data.get('generated_world_sidecar'), dict):
        sidecar = data['generated_world_sidecar']
        sidecar['kind'] = 'directory'
        sidecar['path'] = str(generated_world_sidecar_path)
        sidecar['resolved_path'] = str(generated_world_sidecar_path)
    if isinstance(data.get('world_generation_provenance'), dict):
        provenance = data['world_generation_provenance']
        provenance['kind'] = 'file'
        provenance['path'] = str(world_generation_provenance_path)
        provenance['resolved_path'] = str(world_generation_provenance_path)
        provenance['sha256'] = world_generation_provenance_sha
        provenance['size_bytes'] = world_generation_provenance_size
    data['updated_by'] = updated_by
    bundle_path.write_text(
        json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + '\\n',
        encoding='utf-8',
    )

for genesis_path in genesis_paths:
    data = json.loads(genesis_path.read_text(encoding='utf-8'))
    refs = data.get('governance_bootstrap_refs')
    if isinstance(refs, dict):
        for key, value in list(refs.items()):
            if not isinstance(value, str) or not value.strip():
                continue
            refs[key] = str(config_dir / 'doc' / 'testing' / 'evidence' / Path(value).name)
        genesis_path.write_text(
            json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + '\\n',
            encoding='utf-8',
        )

print(f'synced_validator_signer_count={len(signer_pairs)}')
print(f'synced_runtime_sha256={runtime_sha}')
print(f'synced_generated_world_sidecar={generated_world_sidecar_path}')
print(f'synced_world_generation_provenance_sha256={world_generation_provenance_sha}')
print(f'synced_bundle_count={len(bundle_paths)}')
print(f'synced_genesis_count={len(genesis_paths)}')
PY"
}

import_staged_governance_registry() {
  local host=$1
  local control_path=$2
  ssh_run "$host" "$control_path" \
    "STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

stack_root = Path(os.environ['STACK_ROOT'])
config_dir = stack_root / 'config'
world_dir = stack_root / 'data' / 'execution-world'
import_bin = stack_root / 'current' / 'bin' / 'oasis7_governance_registry_import'

if not import_bin.is_file():
    raise SystemExit(f'missing governance registry import binary: {import_bin}')
if not world_dir.is_dir():
    raise SystemExit(f'missing execution world dir before governance import: {world_dir}')

genesis_paths = sorted(config_dir.rglob('public-testnet-governed-bootstrap-genesis-2026-06-06.json'))
if not genesis_paths:
    raise SystemExit(f'no governed bootstrap genesis found under {config_dir}')

governance_public_manifest = None
for genesis_path in genesis_paths:
    data = json.loads(genesis_path.read_text(encoding='utf-8'))
    refs = data.get('governance_bootstrap_refs')
    if not isinstance(refs, dict):
        continue
    raw = refs.get('governance_public_manifest_ref')
    if isinstance(raw, str) and raw.strip():
        candidate = Path(raw)
        if not candidate.is_absolute():
            candidate = genesis_path.parent / candidate
        if candidate.is_file():
            governance_public_manifest = candidate
            break

if governance_public_manifest is None:
    raise SystemExit(f'no readable governance_public_manifest_ref found under {config_dir}')

command = [
    str(import_bin),
    '--world-dir',
    str(world_dir),
    '--public-manifest',
    str(governance_public_manifest),
]
subprocess.run(command, check=True)
print(f'imported_governance_public_manifest={governance_public_manifest}')
PY"
}

cleanup_host_processes() {
  local host=$1
  local control_path=$2
  local service=$3
  ssh_run "$host" "$control_path" \
    "SERVICE_NAME='$service' STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
import os
import signal
import shlex
import subprocess
import sys
import time

service_name = os.environ['SERVICE_NAME']
stack_root = os.environ['STACK_ROOT']
needles = (
    f'{stack_root}/current/bin/oasis7_chain_runtime',
    f'{stack_root}/bin/start-node.sh',
    f'{stack_root}/releases/',
)

def is_stack_path(value):
    value = (value or '').strip()
    return value == stack_root or value.startswith(f'{stack_root}/')

def unit_metadata_matches_stack_root(metadata):
    for line in metadata.splitlines():
        key, _, value = line.partition('=')
        if key == 'WorkingDirectory':
            if is_stack_path(value):
                return True
        elif key == 'ExecStart':
            for token in shlex.split(value):
                if is_stack_path(token):
                    return True
    return False

def discover_stack_services():
    candidates = {service_name}
    for command in (
        ['systemctl', 'list-unit-files', '--type=service', '--no-legend', '--no-pager'],
        ['systemctl', 'list-units', '--all', '--type=service', '--no-legend', '--no-pager'],
    ):
        out = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
        if out.returncode != 0:
            continue
        for line in out.stdout.splitlines():
            name = line.strip().split(None, 1)[0] if line.strip() else ''
            if name.endswith('.service'):
                candidates.add(name)
    owners = set()
    for candidate in candidates:
        show = subprocess.run(
            ['systemctl', 'show', candidate, '-p', 'FragmentPath', '-p', 'ExecStart', '-p', 'WorkingDirectory'],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if candidate == service_name or (show.returncode == 0 and unit_metadata_matches_stack_root(show.stdout)):
            owners.add(candidate)
    return [service_name] + sorted(owner for owner in owners if owner != service_name)

service_names = discover_stack_services()

def quiesce_systemd():
    for owner in service_names:
        mask = subprocess.run(
            ['systemctl', 'mask', '--runtime', owner],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if mask.returncode != 0:
            details = (mask.stderr or mask.stdout or '').strip()
            suffix = f': {details}' if details else f' (exit {mask.returncode})'
            print(f'cleanup failed: systemctl runtime mask failed for {owner}{suffix}', file=sys.stderr)
            raise SystemExit(1)
        subprocess.run(['systemctl', 'stop', owner], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        subprocess.run(
            ['systemctl', 'kill', '--kill-who=all', '--signal=SIGKILL', owner],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        subprocess.run(['systemctl', 'reset-failed', owner], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)

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

cleanup_deadline_seconds = float(os.environ.get('CLEANUP_DEADLINE_SECONDS', '10'))
cleanup_quiet_seconds = float(os.environ.get('CLEANUP_QUIET_SECONDS', '2'))
deadline = time.monotonic() + cleanup_deadline_seconds
quiet_since = None
quiet_window_observed = False
while time.monotonic() < deadline:
    quiesce_systemd()
    pids = matching_pids()
    if pids:
        quiet_since = None
        for sig in (signal.SIGTERM, signal.SIGKILL):
            for pid in matching_pids():
                try:
                    os.kill(pid, sig)
                except ProcessLookupError:
                    pass
            time.sleep(0.25)
    else:
        if quiet_since is None:
            quiet_since = time.monotonic()
        if time.monotonic() - quiet_since >= cleanup_quiet_seconds:
            quiet_window_observed = True
            break
        time.sleep(0.25)

quiesce_systemd()
remaining = matching_pids()
if remaining:
    print(f'cleanup failed: stack-root processes remain after SIGKILL: {remaining}', file=sys.stderr)
    raise SystemExit(1)
if not quiet_window_observed:
    print('cleanup failed: stable quiet window was not observed before deadline', file=sys.stderr)
    raise SystemExit(1)
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
  ssh_run "$host" "$control_path" "systemctl unmask '$service' || true; systemctl reset-failed '$service' || true; systemctl start '$service'"
}

cleanup_after_failed_start() {
  local label=$1
  local out_path=$2
  if ! cleanup_started_hosts; then
    die "$label failed checks after restart and cleanup failed; see $out_path"
  fi
  die "$label failed checks after restart; see $out_path"
}

SEQUENCER_STARTED=0
STORAGE_STARTED=0

cleanup_started_hosts() {
  local ok=0
  if [[ "$SEQUENCER_STARTED" == 1 ]]; then
    cleanup_host_processes "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE" || ok=1
  fi
  if [[ "$STORAGE_STARTED" == 1 ]]; then
    cleanup_host_processes "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH" "$STORAGE_SERVICE" || ok=1
  fi
  return "$ok"
}

stage_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH"
stage_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH"

reset_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE"
reset_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH" "$STORAGE_SERVICE"

SEQUENCER_STARTED=1
if ! start_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH" "$SEQUENCER_SERVICE"; then
  cleanup_after_failed_start "sequencer start" "$OUT_DIR/sequencer-liveness.json"
fi

STORAGE_STARTED=1
if ! start_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH" "$STORAGE_SERVICE"; then
  cleanup_after_failed_start "storage start" "$OUT_DIR/storage-liveness.json"
fi

poll_status_with_check "$SEQUENCER_STATUS_URL" "$OUT_DIR/sequencer-liveness.json" "sequencer liveness" json_liveness_ok \
  || cleanup_after_failed_start "sequencer liveness" "$OUT_DIR/sequencer-liveness.json"
poll_status_with_check "$STORAGE_STATUS_URL" "$OUT_DIR/storage-liveness.json" "storage liveness" json_liveness_ok \
  || cleanup_after_failed_start "storage liveness" "$OUT_DIR/storage-liveness.json"
poll_status_with_check "$SEQUENCER_STATUS_URL" "$OUT_DIR/sequencer-status.json" "sequencer readiness" json_sequencer_ok \
  || cleanup_after_failed_start "sequencer readiness" "$OUT_DIR/sequencer-status.json"
poll_status_with_check "$STORAGE_STATUS_URL" "$OUT_DIR/storage-status.json" "storage readiness" json_storage_ok \
  || cleanup_after_failed_start "storage readiness" "$OUT_DIR/storage-status.json"

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
