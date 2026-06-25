#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-local-node-install.sh \
    --source-env <path> \
    --source-manifest <path> \
    --runtime-build-ref <path> \
    --node-root <path> \
    [--start-script-source <path>] \
    [--launchd-label <label>] \
    [--load-launchd] \
    [--preserve-state | --reset-state [--state-backup-dir <path>]]

Description:
  Install a local public_testnet observer into a dedicated node directory.
  The installed runtime binary is copied into <node-root>/bin/ and the local
  network-tier bundle copy is rewritten to pin that copied binary hash, so
  normal repo cargo builds cannot drift the running testnet node artifact.

  If <node-root> already contains persisted chain state, the caller must
  choose --preserve-state for a same-chain package upgrade or --reset-state
  for a clean rebuild/redeploy. The default is fail-closed so a validator
  clean rebuild cannot silently leave a local observer on stale chain state.
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

require_non_empty() {
  local flag=$1
  local value=$2
  [[ -n "$value" ]] || die "missing required option: $flag"
}

abs_path() {
  python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).expanduser().resolve())' "$1"
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

source_env=""
source_manifest=""
runtime_build_ref=""
node_root=""
start_script_source="$repo_root/scripts/p2p-triad-node-start.sh"
launchd_label=""
load_launchd=0
state_mode="require-empty"
state_backup_dir=""
state_mode_flag=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-env)
      source_env=${2:-}
      shift 2
      ;;
    --source-manifest)
      source_manifest=${2:-}
      shift 2
      ;;
    --runtime-build-ref)
      runtime_build_ref=${2:-}
      shift 2
      ;;
    --node-root)
      node_root=${2:-}
      shift 2
      ;;
    --start-script-source)
      start_script_source=${2:-}
      shift 2
      ;;
    --launchd-label)
      launchd_label=${2:-}
      shift 2
      ;;
    --load-launchd)
      load_launchd=1
      shift
      ;;
    --preserve-state)
      [[ -z "$state_mode_flag" ]] || die "--preserve-state conflicts with $state_mode_flag"
      state_mode="preserve"
      state_mode_flag="--preserve-state"
      shift
      ;;
    --reset-state)
      [[ -z "$state_mode_flag" ]] || die "--reset-state conflicts with $state_mode_flag"
      state_mode="reset"
      state_mode_flag="--reset-state"
      shift
      ;;
    --state-backup-dir)
      state_backup_dir=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

require_non_empty "--source-env" "$source_env"
require_non_empty "--source-manifest" "$source_manifest"
require_non_empty "--runtime-build-ref" "$runtime_build_ref"
require_non_empty "--node-root" "$node_root"
require_file "$source_env"
require_file "$source_manifest"
require_file "$runtime_build_ref"
require_file "$start_script_source"

source_env=$(abs_path "$source_env")
source_manifest=$(abs_path "$source_manifest")
runtime_build_ref=$(abs_path "$runtime_build_ref")
node_root=$(abs_path "$node_root")
start_script_source=$(abs_path "$start_script_source")
if [[ -n "$state_backup_dir" ]]; then
  state_backup_dir=$(abs_path "$state_backup_dir")
fi

if [[ -n "$state_backup_dir" && "$state_mode" != "reset" ]]; then
  die "--state-backup-dir requires --reset-state"
fi

path_has_state() {
  local path=$1
  [[ -e "$path" ]] || return 1
  if [[ -d "$path" ]]; then
    find "$path" -mindepth 1 -maxdepth 1 -print -quit | grep -q .
    return
  fi
  return 0
}

state_paths=(
  "$node_root/world"
  "$node_root/world-simulator-mirror"
  "$node_root/execution-records"
  "$node_root/store"
  "$node_root/replication-root"
  "$node_root/runtime-root"
  "$node_root/output/chain-runtime"
)
existing_state_paths=()
for state_path in "${state_paths[@]}"; do
  if path_has_state "$state_path"; then
    existing_state_paths+=("$state_path")
  fi
done

if [[ "$state_mode" == "require-empty" && ${#existing_state_paths[@]} -gt 0 ]]; then
  {
    printf 'error: local observer node root contains persisted chain state:\n'
    printf '  %s\n' "${existing_state_paths[@]}"
    printf 'choose --preserve-state for a same-chain package upgrade, or --reset-state for a clean rebuild/redeploy\n'
  } >&2
  exit 1
fi

if [[ "$state_mode" == "reset" && ${#existing_state_paths[@]} -gt 0 ]]; then
  if [[ -z "$state_backup_dir" ]]; then
    state_backup_dir="$node_root/backups/local-node-install-state-reset-$(date -u +%Y%m%dT%H%M%SZ)"
  fi
  [[ ! -e "$state_backup_dir" ]] || die "state backup dir already exists: $state_backup_dir"
  mkdir -p "$state_backup_dir"
  for state_path in "${existing_state_paths[@]}"; do
    mv "$state_path" "$state_backup_dir/$(basename "$state_path")"
    printf 'backed up stale local observer state: %s -> %s\n' "$state_path" "$state_backup_dir/$(basename "$state_path")"
  done
fi

mkdir -p "$node_root/bin" "$node_root/config" "$node_root/logs"
install -m 0755 "$runtime_build_ref" "$node_root/bin/oasis7_chain_runtime"
install -m 0755 "$start_script_source" "$node_root/bin/start-node.sh"

python3 - "$source_env" "$source_manifest" "$node_root" <<'PY'
from __future__ import annotations

import hashlib
import json
import os
import shutil
import string
import sys
from pathlib import Path

source_env = Path(sys.argv[1])
source_manifest = Path(sys.argv[2])
node_root = Path(sys.argv[3])


def parse_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        merged_env = {**os.environ, **values}
        values[key] = string.Template(value.strip()).safe_substitute(merged_env)
    return values


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def copy_if_file(source: Path, target: Path) -> None:
    if source.is_file():
        if source.resolve() == target.resolve():
            return
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)


def resolve_ref(manifest_path: Path, raw_ref: str) -> Path:
    candidate = Path(raw_ref).expanduser()
    if candidate.is_absolute():
        return candidate
    return manifest_path.parent / candidate


env_values = parse_env(source_env)
source_stack_root = Path(env_values.get("STACK_ROOT", source_env.parent)).expanduser()
if not source_stack_root.is_absolute():
    source_stack_root = (source_env.parent / source_stack_root).resolve()
source_stack_root = source_stack_root.resolve()

runtime_bin = node_root / "bin" / "oasis7_chain_runtime"
runtime_sha = sha256_file(runtime_bin)

manifest = json.loads(source_manifest.read_text(encoding="utf-8"))
runtime_refs = manifest.setdefault("runtime_refs", {})

for key in ("release_candidate_bundle_ref", "genesis_ref", "bootstrap_peer_ref"):
    raw_ref = runtime_refs.get(key)
    if not raw_ref:
        continue
    source = resolve_ref(source_manifest, str(raw_ref)).resolve()
    if not source.is_file():
        raise SystemExit(f"manifest runtime ref source missing: {source}")
    target = node_root / source.name
    copy_if_file(source, target)
    runtime_refs[key] = target.name

genesis_ref = runtime_refs.get("genesis_ref")
if genesis_ref:
    genesis_path = node_root / str(genesis_ref)
    genesis = json.loads(genesis_path.read_text(encoding="utf-8"))
    bootstrap_refs = genesis.get("governance_bootstrap_refs", {})
    if isinstance(bootstrap_refs, dict):
        for raw_ref in bootstrap_refs.values():
            if not isinstance(raw_ref, str) or not raw_ref:
                continue
            source = Path(raw_ref).expanduser()
            if not source.is_absolute():
                source = Path.cwd() / source
            if source.is_file():
                target = node_root / raw_ref
                copy_if_file(source, target)

bundle_ref = runtime_refs.get("release_candidate_bundle_ref")
if not bundle_ref:
    raise SystemExit("manifest missing runtime_refs.release_candidate_bundle_ref")
bundle_path = node_root / str(bundle_ref)
bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
runtime_build = bundle.setdefault("runtime_build", {})
runtime_build["path"] = str(runtime_bin)
runtime_build["ref"] = "bin/oasis7_chain_runtime"
runtime_build["resolved_path"] = str(runtime_bin)
runtime_build["sha256"] = runtime_sha
runtime_build["size_bytes"] = runtime_bin.stat().st_size
runtime_build["updated_by"] = "p2p-public-testnet-local-node-install dedicated local node artifact"
bundle_path.write_text(json.dumps(bundle, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")

(node_root / "manifest.json").write_text(
    json.dumps(manifest, ensure_ascii=True, indent=2) + "\n",
    encoding="utf-8",
)

copy_if_file(Path(env_values.get("CONFIG_PATH", source_stack_root / "config.toml")), node_root / "config.toml")
copy_if_file(
    Path(env_values.get("GENESIS_VALIDATOR_REGISTRY_PATH", source_stack_root / "config" / "genesis-validator-registry.json")),
    node_root / "config" / "genesis-validator-registry.json",
)

rewrites = {
    "STACK_ROOT": str(node_root),
    "CONFIG_PATH": str(node_root / "config.toml"),
    "RUNTIME_ROOT": str(node_root / "runtime-root"),
    "EXECUTION_WORLD_DIR": str(node_root / "world"),
    "EXECUTION_RECORDS_DIR": str(node_root / "execution-records"),
    "STORAGE_ROOT": str(node_root / "store"),
    "REPLICATION_ROOT": str(node_root / "replication-root"),
    "NETWORK_TIER_MANIFEST_PATH": str(node_root / "manifest.json"),
    "GENESIS_VALIDATOR_REGISTRY_PATH": str(node_root / "config" / "genesis-validator-registry.json"),
    "TRAFFIC_MONITOR_OUTPUT_DIR": str(node_root / "output" / "traffic-monitor"),
    "BIN": str(runtime_bin),
}

existing_lines = source_env.read_text(encoding="utf-8").splitlines()
seen: set[str] = set()
rendered: list[str] = []
for raw_line in existing_lines:
    if "=" not in raw_line or raw_line.lstrip().startswith("#"):
        rendered.append(raw_line)
        continue
    key = raw_line.split("=", 1)[0]
    if key in rewrites:
        rendered.append(f"{key}={rewrites[key]}")
        seen.add(key)
    else:
        rendered.append(raw_line)

for key, value in rewrites.items():
    if key not in seen:
        rendered.append(f"{key}={value}")

(node_root / "node.env").write_text("\n".join(rendered) + "\n", encoding="utf-8")
PY

plist_path=""
if [[ -n "$launchd_label" ]]; then
  plist_path="$node_root/$launchd_label.plist"
  python3 - "$plist_path" "$launchd_label" "$node_root" <<'PY'
from __future__ import annotations

import plistlib
import sys
from pathlib import Path

plist_path = Path(sys.argv[1])
label = sys.argv[2]
node_root = Path(sys.argv[3])
payload = {
    "Label": label,
    "ProgramArguments": [
        "/usr/bin/env",
        f"APP_ROOT={node_root}",
        f"ENV_FILE={node_root / 'node.env'}",
        f"BIN={node_root / 'bin' / 'oasis7_chain_runtime'}",
        str(node_root / "bin" / "start-node.sh"),
    ],
    "RunAtLoad": True,
    "KeepAlive": True,
    "StandardOutPath": str(node_root / "logs" / "launchd.out.log"),
    "StandardErrorPath": str(node_root / "logs" / "launchd.err.log"),
    "WorkingDirectory": str(node_root),
}
plist_path.write_bytes(plistlib.dumps(payload, fmt=plistlib.FMT_XML, sort_keys=True))
PY
fi

if [[ "$load_launchd" -eq 1 ]]; then
  [[ -n "$launchd_label" ]] || die "--load-launchd requires --launchd-label"
  launchctl bootout "gui/$(id -u)/$launchd_label" >/dev/null 2>&1 || true
  launchctl bootstrap "gui/$(id -u)" "$plist_path"
fi

printf 'installed local testnet node root: %s\n' "$node_root"
printf 'runtime: %s\n' "$node_root/bin/oasis7_chain_runtime"
printf 'env: %s\n' "$node_root/node.env"
printf 'manifest: %s\n' "$node_root/manifest.json"
if [[ -n "$plist_path" ]]; then
  printf 'launchd_plist: %s\n' "$plist_path"
fi
