#!/usr/bin/env bash
set -euo pipefail

provider_cli_bin="${OASIS7_PROVIDER_CLI_BIN:-$(printf %s "open""claw")}"
usage() {
  cat <<'USAGE'
Usage:
  oasis7-run.sh download [options]
  oasis7-run.sh play [options]
  oasis7-run.sh smoke [options]
  oasis7-run.sh doctor [options]

Options:
  --repo-root <path>              Explicit repo root for repo-backed actions
  --bundle-dir <path>             Extracted release bundle root containing run-game.sh
  --download-release              Download oasis7 bundle from GitHub Release before play
  --release-platform <id>         Release asset platform: linux-x64|macos-x64|windows-x64
  --release-tag <tag>             GitHub release tag or latest (default: latest)
  --release-repo <owner/repo>     GitHub repo slug for release download (default: eng-cc/oasis7)
  --download-dir <path>           Release cache/output root (default: ~/.cache/oasis7/releases)
  --force-download                Redownload bundle even if cached bundle already exists
  --base-url <url>                local provider base url (default: http://127.0.0.1:5841)
  --agent-id <id>                 local provider runtime agent id (default: oasis7_provider_agent)
  --agent-profile <profile>       local provider agent profile (default: oasis7_p0_low_freq_npc)
  --execution-mode <mode>         local provider execution mode (default: headless_agent)
  --scenario <name>               Gameplay scenario (default: llm_bootstrap)
  --timeout-ms <ms>               Smoke timeout budget (default: 15000)
  --connect-timeout-ms <ms>       Provider connect timeout (default: 15000)
  --samples <n>                   Smoke samples (default: 1)
  --ticks <n>                     Smoke ticks (default: 4)
  --bridge-log <path>             Bridge log path (default: <repo>/.tmp/oasis7-bridge.log or ./.tmp/oasis7-bridge.log)
  --skip-agent-setup              Skip runtime agent bootstrap
  --reuse-bridge                  Reuse existing bridge at --base-url
  --no-open-browser               Pass through to oasis7_game_launcher
  --json                          Emit machine-readable JSON for doctor mode
  -h, --help                      Show help
USAGE
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

http_get() {
  local url="$1"
  curl -fsS "$url"
}

stderr_is_tty() {
  [[ -t 2 ]]
}

file_size_bytes() {
  local path="$1"
  wc -c <"$path" | tr -d '[:space:]'
}

run_with_heartbeat() {
  local label="$1"
  shift

  local heartbeat_s="${OASIS7_DOWNLOAD_HEARTBEAT_SECS:-10}"
  if ! [[ "$heartbeat_s" =~ ^[0-9]+$ ]] || [[ "$heartbeat_s" -le 0 ]]; then
    heartbeat_s=10
  fi

  if stderr_is_tty; then
    "$@"
    return $?
  fi

  "$@" &
  local cmd_pid="$!"
  local start_s
  start_s="$(date +%s 2>/dev/null || printf "0")"

  while kill -0 "$cmd_pid" >/dev/null 2>&1; do
    sleep "$heartbeat_s"
    if ! kill -0 "$cmd_pid" >/dev/null 2>&1; then
      break
    fi
    local now_s elapsed_s
    now_s="$(date +%s 2>/dev/null || printf "0")"
    if [[ "$start_s" =~ ^[0-9]+$ && "$now_s" =~ ^[0-9]+$ && "$now_s" -ge "$start_s" ]]; then
      elapsed_s=$((now_s - start_s))
      printf '%s (elapsed=%ss)\n' "$label" "$elapsed_s" >&2
    else
      printf '%s\n' "$label" >&2
    fi
  done

  wait "$cmd_pid"
}

curl_download_file() {
  local url="$1"
  local output_path="$2"
  local heartbeat_label="$3"
  local curl_args=(-L --fail --show-error -o "$output_path")

  if stderr_is_tty; then
    curl_args+=(--progress-bar)
    curl "${curl_args[@]}" "$url"
    return $?
  fi

  curl_args+=(--silent)
  run_with_heartbeat "$heartbeat_label" curl "${curl_args[@]}" "$url"
}

wait_for_http() {
  local url="$1"
  local attempts="${2:-40}"
  local sleep_s="${3:-0.5}"
  local i
  for ((i=0; i<attempts; i+=1)); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$sleep_s"
  done
  echo "timed out waiting for $url" >&2
  return 1
}

encode_b64() {
  python - <<'PY' "$1"
import base64, sys
print(base64.b64encode(sys.argv[1].encode()).decode())
PY
}

print_doctor_status() {
  local level="$1"
  local label="$2"
  local detail="$3"
  if [[ "$json_output" != "1" ]]; then
    printf '[%s] %s: %s\n' "$level" "$label" "$detail"
  fi
  if [[ -n "$doctor_records_file" ]]; then
    printf '%s\t%s\t%s\n' "$level" "$label" "$(encode_b64 "$detail")" >>"$doctor_records_file"
  fi
}

process_alive() {
  local pid="$1"
  kill -0 "$pid" >/dev/null 2>&1
}

process_group_alive() {
  local pgid="$1"
  kill -0 -- "-$pgid" >/dev/null 2>&1
}

terminate_pid() {
  local pid="$1"
  local signal="$2"
  kill -s "$signal" "$pid" >/dev/null 2>&1 || true
}

terminate_process_group() {
  local pgid="$1"
  local signal="$2"
  kill -s "$signal" -- "-$pgid" >/dev/null 2>&1 || true
}

wait_for_pid_exit() {
  local pid="$1"
  local attempts="${2:-20}"
  local sleep_s="${3:-0.1}"
  local i
  for ((i=0; i<attempts; i+=1)); do
    if ! process_alive "$pid"; then
      return 0
    fi
    sleep "$sleep_s"
  done
  return 1
}

wait_for_process_group_exit() {
  local pgid="$1"
  local attempts="${2:-20}"
  local sleep_s="${3:-0.1}"
  local i
  for ((i=0; i<attempts; i+=1)); do
    if ! process_group_alive "$pgid"; then
      return 0
    fi
    sleep "$sleep_s"
  done
  return 1
}

cleanup_process_tree() {
  local pid="$1"
  local pgid="$2"
  if [[ -n "$pgid" && "$pgid" =~ ^[0-9]+$ ]]; then
    terminate_process_group "$pgid" TERM
    if ! wait_for_process_group_exit "$pgid" 20 0.1; then
      terminate_process_group "$pgid" KILL
      wait_for_process_group_exit "$pgid" 20 0.1 || true
    fi
  elif [[ -n "$pid" && "$pid" =~ ^[0-9]+$ ]]; then
    terminate_pid "$pid" TERM
    if ! wait_for_pid_exit "$pid" 20 0.1; then
      terminate_pid "$pid" KILL
      wait_for_pid_exit "$pid" 20 0.1 || true
    fi
  fi
  if [[ -n "$pid" && "$pid" =~ ^[0-9]+$ ]]; then
    wait "$pid" >/dev/null 2>&1 || true
  fi
}

expand_current_user_home_path() {
  local value="$1"
  if [[ "$value" == "~" ]]; then
    printf '%s' "$HOME"
  elif [[ "${value:0:2}" == "~/" ]]; then
    printf '%s/%s' "$HOME" "${value:2}"
  else
    printf '%s' "$value"
  fi
}

normalize_path() {
  local value="$1"
  local expanded
  expanded="$(expand_current_user_home_path "$value")"
  if [[ "$expanded" == /* ]]; then
    printf '%s' "$expanded"
  else
    printf '%s/%s' "$PWD" "$expanded"
  fi
}

resolve_source_tree_viewer_static_dir() {
  local repo_root="$1"
  local out_dir="$2"

  # shellcheck source=/dev/null
  source "$repo_root/scripts/agent-browser-lib.sh"
  resolve_viewer_static_dir_for_web_closure "$repo_root" "web" "$out_dir"
}

validate_repo_root() {
  local candidate="$1"
  [[ -f "$candidate/Cargo.toml" ]] &&
    [[ -f "$candidate/scripts/setup-provider-oasis7-runtime.sh" ]] &&
    [[ -f "$candidate/scripts/provider-parity-p0.sh" ]]
}

search_repo_root_upwards() {
  local dir="$1"
  while [[ -n "$dir" && "$dir" != "/" ]]; do
    if validate_repo_root "$dir"; then
      printf '%s\n' "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  if validate_repo_root "/"; then
    printf '/\n'
    return 0
  fi
  return 1
}

discover_repo_root() {
  local candidate=""
  if [[ -n "$repo_root_override" ]]; then
    candidate="$(normalize_path "$repo_root_override")"
    if validate_repo_root "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
    echo "error: invalid --repo-root, missing repo markers: $candidate" >&2
    return 1
  fi

  if candidate="$(git rev-parse --show-toplevel 2>/dev/null)" && validate_repo_root "$candidate"; then
    printf '%s\n' "$candidate"
    return 0
  fi

  if candidate="$(search_repo_root_upwards "$PWD" 2>/dev/null)"; then
    printf '%s\n' "$candidate"
    return 0
  fi

  local script_dir
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  if candidate="$(search_repo_root_upwards "$script_dir" 2>/dev/null)"; then
    printf '%s\n' "$candidate"
    return 0
  fi

  return 1
}

resolve_bridge_log_default() {
  local root="$1"
  if [[ -n "$bridge_log_override" ]]; then
    printf '%s\n' "$(normalize_path "$bridge_log_override")"
  elif [[ -n "$root" ]]; then
    printf '%s\n' "$root/.tmp/oasis7-bridge.log"
  else
    printf '%s\n' "$PWD/.tmp/oasis7-bridge.log"
  fi
}

validate_bundle_dir() {
  local candidate="$1"
  [[ -x "$candidate/run-game.sh" ]]
}

detect_release_platform() {
  local uname_s
  uname_s="$(uname -s)"
  case "$uname_s" in
    Linux)
      printf 'linux-x64\n'
      ;;
    Darwin)
      printf 'macos-x64\n'
      ;;
    MINGW*|MSYS*|CYGWIN*)
      printf 'windows-x64\n'
      ;;
    *)
      echo "error: unsupported host platform '$uname_s'; pass --release-platform explicitly" >&2
      return 1
      ;;
  esac
}

release_asset_name() {
  case "$1" in
    linux-x64)
      printf 'oasis7-linux-x64.tar.gz\n'
      ;;
    macos-x64)
      printf 'oasis7-macos-x64.tar.gz\n'
      ;;
    windows-x64)
      printf 'oasis7-windows-x64.zip\n'
      ;;
    *)
      echo "error: unsupported --release-platform: $1" >&2
      return 1
      ;;
  esac
}

release_download_url() {
  local repo="$1"
  local tag="$2"
  local asset="$3"
  if [[ "$tag" == "latest" ]]; then
    printf 'https://github.com/%s/releases/latest/download/%s\n' "$repo" "$asset"
  else
    printf 'https://github.com/%s/releases/download/%s/%s\n' "$repo" "$tag" "$asset"
  fi
}

verify_release_checksum() {
  local checksum_path="$1"
  local archive_path="$2"
  python - <<'PY' "$checksum_path" "$archive_path"
import hashlib, pathlib, sys
checksum_path = pathlib.Path(sys.argv[1])
archive_path = pathlib.Path(sys.argv[2])
expected = None
with checksum_path.open('r', encoding='utf-8') as handle:
    for raw in handle:
        parts = raw.strip().split()
        if len(parts) >= 2 and parts[-1] == archive_path.name:
            expected = parts[0]
            break
if expected is None:
    sys.exit(2)
sha = hashlib.sha256()
with archive_path.open('rb') as handle:
    while True:
        chunk = handle.read(1024 * 1024)
        if not chunk:
            break
        sha.update(chunk)
actual = sha.hexdigest()
if actual != expected:
    print(f'checksum mismatch for {archive_path.name}: expected {expected}, got {actual}', file=sys.stderr)
    sys.exit(1)
print(actual)
PY
}

find_extracted_bundle_dir() {
  local extract_root="$1"
  local platform="$2"
  if validate_bundle_dir "$extract_root"; then
    printf '%s\n' "$extract_root"
    return 0
  fi
  if validate_bundle_dir "$extract_root/oasis7-$platform"; then
    printf '%s\n' "$extract_root/oasis7-$platform"
    return 0
  fi
  local marker
  marker="$(find "$extract_root" -maxdepth 3 -type f -name run-game.sh | head -n 1 || true)"
  if [[ -n "$marker" ]]; then
    dirname "$marker"
    return 0
  fi
  echo "error: extracted release bundle does not contain run-game.sh under $extract_root" >&2
  return 1
}

download_release_bundle() {
  require_cmd curl

  local platform="$release_platform"
  if [[ -z "$platform" ]]; then
    platform="$(detect_release_platform)"
  fi
  local asset_name
  asset_name="$(release_asset_name "$platform")"

  local cache_root
  cache_root="$(normalize_path "$download_dir")"
  local repo_key="${release_repo//\//-}"
  local target_root="$cache_root/$repo_key/$release_tag/$platform"
  local bundle_root="$target_root/bundle"
  local archive_path="$target_root/$asset_name"
  local checksum_path="$target_root/oasis7-checksums.txt"
  local extract_root="$target_root/extracted"
  local asset_url
  asset_url="$(release_download_url "$release_repo" "$release_tag" "$asset_name")"
  local checksum_url
  checksum_url="$(release_download_url "$release_repo" "$release_tag" "oasis7-checksums.txt")"

  if [[ "$force_download" != "1" && -x "$bundle_root/run-game.sh" ]]; then
    echo "Reusing cached release bundle: $bundle_root" >&2
    printf '%s\n' "$bundle_root"
    return 0
  fi

  rm -rf "$target_root"
  mkdir -p "$target_root" "$extract_root"

  echo "Downloading release asset: $asset_url" >&2
  echo "- target archive: $archive_path" >&2
  curl_download_file "$asset_url" "$archive_path" "Downloading release asset…"
  local archive_size_bytes
  archive_size_bytes="$(file_size_bytes "$archive_path")"
  echo "Downloaded archive: $archive_path (bytes=$archive_size_bytes)" >&2

  echo "Fetching release checksums: $checksum_url" >&2
  if curl_download_file "$checksum_url" "$checksum_path" "Fetching release checksums…"; then
    if checksum_value="$(verify_release_checksum "$checksum_path" "$archive_path" 2>/dev/null)"; then
      echo "Verified SHA256: $checksum_value" >&2
    else
      status=$?
      if [[ "$status" -eq 1 ]]; then
        echo "error: release checksum verification failed for $archive_path" >&2
        exit 1
      fi
      echo "warning: checksums file did not contain $asset_name; skipped verification" >&2
    fi
  else
    rm -f "$checksum_path"
    echo "warning: could not download release checksums; skipped verification" >&2
  fi

  [[ -f "$archive_path" ]] || {
    echo "error: release archive missing before extraction: $archive_path" >&2
    exit 1
  }

  echo "Extracting bundle archive into: $extract_root" >&2
  case "$asset_name" in
    *.tar.gz)
      require_cmd tar
      tar -xzf "$archive_path" -C "$extract_root"
      ;;
    *.zip)
      require_cmd unzip
      unzip -q "$archive_path" -d "$extract_root"
      ;;
    *)
      echo "error: unsupported release archive format: $asset_name" >&2
      exit 1
      ;;
  esac

  local detected_bundle
  if ! detected_bundle="$(find_extracted_bundle_dir "$extract_root" "$platform")"; then
    echo "error: bundle detection failed; refusing to populate cache bundle dir from an unresolved path" >&2
    exit 1
  fi
  [[ -n "$detected_bundle" ]] || {
    echo "error: bundle detection returned an empty path; refusing to populate cache bundle dir" >&2
    exit 1
  }
  [[ "$detected_bundle" == /* ]] || {
    echo "error: bundle detection returned a non-absolute path: $detected_bundle" >&2
    exit 1
  }
  [[ -x "$detected_bundle/run-game.sh" ]] || {
    echo "error: detected bundle is missing run-game.sh: $detected_bundle" >&2
    exit 1
  }
  echo "Preparing bundle directory: $bundle_root" >&2
  rm -rf "$bundle_root"
  mkdir -p "$bundle_root"
  cp -R "$detected_bundle/." "$bundle_root/"
  echo "Bundle ready: $bundle_root" >&2
  printf '%s\n' "$bundle_root"
}

emit_doctor_json() {
  local failures="$1"
  python - <<'PY' "$doctor_records_file" "$failures" "$base_url" "$agent_id" "$agent_profile" "$scenario"
import base64, json, sys
records_path, failures, base_url, agent_id, agent_profile, scenario = sys.argv[1:]
checks = []
with open(records_path, "r", encoding="utf-8") as handle:
    for line in handle:
        level, label, detail_b64 = line.rstrip("\n").split("\t", 2)
        detail = base64.b64decode(detail_b64.encode()).decode()
        checks.append({"level": level, "label": label, "detail": detail})
print(json.dumps({
    "ok": int(failures) == 0,
    "failures": int(failures),
    "base_url": base_url,
    "agent_id": agent_id,
    "agent_profile": agent_profile,
    "scenario": scenario,
    "checks": checks,
}, ensure_ascii=False))
PY
}

run_doctor() {
  local failures=0
  local gateway_json=""
  local provider_info_json=""
  local agents_json=""
  local resolved_repo_root=""
  local resolved_bundle_dir=""
  local cargo_available="0"
  local bridge_health_ok="0"
  doctor_records_file="$(mktemp)"

  print_doctor_status INFO config "base_url=$base_url agent_id=$agent_id agent_profile=$agent_profile scenario=$scenario release_repo=$release_repo release_tag=$release_tag"

  if resolved_repo_root="$(discover_repo_root 2>/dev/null)"; then
    print_doctor_status OK repo-root "$resolved_repo_root"
  else
    print_doctor_status INFO repo-root "not resolved (needed only for agent bootstrap, bridge launch, or smoke)"
  fi

  if [[ -n "$bundle_dir" ]]; then
    resolved_bundle_dir="$(normalize_path "$bundle_dir")"
    if validate_bundle_dir "$resolved_bundle_dir"; then
      print_doctor_status OK bundle-dir "$resolved_bundle_dir"
    else
      print_doctor_status FAIL bundle-dir "missing run-game.sh under $resolved_bundle_dir"
      failures=$((failures + 1))
    fi
  elif [[ "$download_release" == "1" ]]; then
    print_doctor_status INFO bundle-dir "download on demand enabled (platform=${release_platform:-auto}, cache=$(normalize_path "$download_dir"))"
  else
    print_doctor_status INFO bundle-dir "not configured"
  fi

  if command -v "$provider_cli_bin" >/dev/null 2>&1; then
    print_doctor_status OK command "provider-cli=$(command -v "$provider_cli_bin")"
  else
    print_doctor_status FAIL command "provider runtime CLI not found"
    failures=$((failures + 1))
  fi

  if command -v cargo >/dev/null 2>&1; then
    cargo_available="1"
    print_doctor_status OK command "cargo=$(command -v cargo)"
  else
    print_doctor_status WARN command "cargo not found (repo-backed bridge/bootstrap unavailable; bundle-first play can still reuse an existing bridge via --reuse-bridge --skip-agent-setup)"
  fi

  if gateway_json="$(http_get 'http://127.0.0.1:18789/health' 2>/dev/null)"; then
    print_doctor_status OK gateway "$gateway_json"
  else
    print_doctor_status FAIL gateway "cannot reach http://127.0.0.1:18789/health"
    failures=$((failures + 1))
  fi

  if agents_json="$("$provider_cli_bin" agents list --json 2>/dev/null)"; then
    if AGENTS_JSON="$agents_json" AGENT_ID="$agent_id" python - <<'PY' >/dev/null
import json, os, sys
agent_id = os.environ['AGENT_ID']
items = json.loads(os.environ['AGENTS_JSON'])
for item in items:
    if item.get('id') == agent_id:
        sys.exit(0)
sys.exit(1)
PY
    then
      local agent_summary
      agent_summary="$(AGENTS_JSON="$agents_json" AGENT_ID="$agent_id" python - <<'PY'
import json, os
agent_id = os.environ['AGENT_ID']
items = json.loads(os.environ['AGENTS_JSON'])
for item in items:
    if item.get('id') == agent_id:
        print(f"workspace={item.get('workspace','')} model={item.get('model','')}")
        break
PY
)"
      print_doctor_status OK runtime-agent "$agent_summary"
    else
      if [[ -n "$resolved_repo_root" ]]; then
        print_doctor_status FAIL runtime-agent "local provider agent '$agent_id' not found; run $resolved_repo_root/scripts/setup-provider-oasis7-runtime.sh $agent_id"
      else
        print_doctor_status FAIL runtime-agent "local provider agent '$agent_id' not found; provide --repo-root and run scripts/setup-provider-oasis7-runtime.sh $agent_id"
      fi
      failures=$((failures + 1))
    fi
  else
    print_doctor_status FAIL runtime-agent "failed to query provider runtime agent inventory"
    failures=$((failures + 1))
  fi

  if http_get "$base_url/v1/provider/health" >/dev/null 2>&1; then
    bridge_health_ok="1"
    print_doctor_status OK bridge-health "$base_url/v1/provider/health reachable"
  else
    print_doctor_status FAIL bridge-health "cannot reach $base_url/v1/provider/health"
    failures=$((failures + 1))
  fi

  if provider_info_json="$(http_get "$base_url/v1/provider/info" 2>/dev/null)"; then
    local provider_summary
    provider_summary="$(PROVIDER_INFO_JSON="$provider_info_json" python - <<'PY'
import json, os
value = json.loads(os.environ['PROVIDER_INFO_JSON'])
provider_id = value.get('provider_id', '')
provider_version = value.get('provider_version', '')
protocol_version = value.get('protocol_version', '')
print(f"provider_id={provider_id} provider_version={provider_version} protocol_version={protocol_version}")
PY
)"
    print_doctor_status OK provider-info "$provider_summary"
  else
    print_doctor_status FAIL provider-info "cannot reach $base_url/v1/provider/info"
    failures=$((failures + 1))
  fi

  if [[ -n "$resolved_repo_root" && "$cargo_available" == "1" ]]; then
    print_doctor_status OK repo-bootstrap "repo-backed bridge/bootstrap available"
  else
    local repo_bootstrap_reason=""
    if [[ -z "$resolved_repo_root" && "$cargo_available" != "1" ]]; then
      repo_bootstrap_reason="repo root not resolved and cargo not found"
    elif [[ -z "$resolved_repo_root" ]]; then
      repo_bootstrap_reason="repo root not resolved"
    else
      repo_bootstrap_reason="cargo not found"
    fi
    print_doctor_status WARN repo-bootstrap "$repo_bootstrap_reason; auto bridge/runtime bootstrap needs repo root + cargo. Bundle-first no-cargo play can reuse an existing bridge via --reuse-bridge --skip-agent-setup"
  fi

  if [[ -n "$resolved_bundle_dir" ]]; then
    if [[ "$bridge_health_ok" == "1" ]]; then
      print_doctor_status OK bundle-play "bundle-first no-cargo play ready; run play --bundle-dir $resolved_bundle_dir --reuse-bridge --skip-agent-setup"
    else
      print_doctor_status WARN bundle-play "bundle is valid, but no bridge is reachable at $base_url; start or reuse a bridge, then run play --bundle-dir $resolved_bundle_dir --reuse-bridge --skip-agent-setup"
    fi
  elif [[ "$download_release" == "1" ]]; then
    if [[ "$bridge_health_ok" == "1" ]]; then
      print_doctor_status INFO bundle-play "download-on-demand + running bridge can support no-cargo play via --download-release --reuse-bridge --skip-agent-setup"
    else
      print_doctor_status INFO bundle-play "download-on-demand enabled; bridge still needs to be reachable at $base_url for no-cargo bundle play"
    fi
  else
    print_doctor_status INFO bundle-play "not configured; pass --bundle-dir <path> or --download-release to evaluate bundle-first no-cargo readiness"
  fi

  if [[ -f "$bridge_log" ]]; then
    print_doctor_status INFO bridge-log "$bridge_log"
  else
    print_doctor_status INFO bridge-log "not created yet ($bridge_log)"
  fi

  if [[ "$failures" -eq 0 ]]; then
    print_doctor_status OK summary "doctor checks passed"
    if [[ "$json_output" == "1" ]]; then
      emit_doctor_json "$failures"
    fi
    return 0
  fi

  print_doctor_status FAIL summary "$failures check(s) failed"
  if [[ "$json_output" == "1" ]]; then
    emit_doctor_json "$failures"
  fi
  return 1
}

main() {
mode="${1:-}"
if [[ -z "$mode" || "$mode" == "-h" || "$mode" == "--help" ]]; then
  usage
  exit 0
fi
shift

repo_root_override=""
bundle_dir=""
download_release="0"
release_platform=""
release_tag="latest"
release_repo="eng-cc/oasis7"
download_dir="~/.cache/oasis7/releases"
force_download="0"
base_url="http://127.0.0.1:5841"
agent_id="oasis7_provider_agent"
agent_profile="oasis7_p0_low_freq_npc"
execution_mode="headless_agent"
scenario="llm_bootstrap"
timeout_ms="15000"
connect_timeout_ms="15000"
samples="1"
ticks="4"
bridge_log_override=""
skip_agent_setup="0"
reuse_bridge="0"
open_browser="1"
json_output="0"
doctor_records_file=""
repo_root=""
bridge_log=""
cleanup_bridge_pid=""
cleanup_play_pid=""
cleanup_play_pgid=""
cleanup_done="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      repo_root_override="$2"
      shift 2
      ;;
    --bundle-dir)
      bundle_dir="$2"
      shift 2
      ;;
    --download-release)
      download_release="1"
      shift
      ;;
    --release-platform)
      release_platform="$2"
      shift 2
      ;;
    --release-tag)
      release_tag="$2"
      shift 2
      ;;
    --release-repo)
      release_repo="$2"
      shift 2
      ;;
    --download-dir)
      download_dir="$2"
      shift 2
      ;;
    --force-download)
      force_download="1"
      shift
      ;;
    --base-url)
      base_url="$2"
      shift 2
      ;;
    --agent-id)
      agent_id="$2"
      shift 2
      ;;
    --agent-profile)
      agent_profile="$2"
      shift 2
      ;;
    --execution-mode)
      execution_mode="$2"
      shift 2
      ;;
    --scenario)
      scenario="$2"
      shift 2
      ;;
    --timeout-ms)
      timeout_ms="$2"
      shift 2
      ;;
    --connect-timeout-ms)
      connect_timeout_ms="$2"
      shift 2
      ;;
    --samples)
      samples="$2"
      shift 2
      ;;
    --ticks)
      ticks="$2"
      shift 2
      ;;
    --bridge-log)
      bridge_log_override="$2"
      shift 2
      ;;
    --skip-agent-setup)
      skip_agent_setup="1"
      shift
      ;;
    --reuse-bridge)
      reuse_bridge="1"
      shift
      ;;
    --no-open-browser)
      open_browser="0"
      shift
      ;;
    --json)
      json_output="1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

cleanup() {
  if [[ "$cleanup_done" == "1" ]]; then
    return
  fi
  cleanup_done="1"
  if [[ -n "$cleanup_play_pid" ]]; then
    cleanup_process_tree "$cleanup_play_pid" "$cleanup_play_pgid"
    cleanup_play_pid=""
    cleanup_play_pgid=""
  fi
  if [[ -n "$cleanup_bridge_pid" ]]; then
    kill "$cleanup_bridge_pid" >/dev/null 2>&1 || true
    wait "$cleanup_bridge_pid" >/dev/null 2>&1 || true
    cleanup_bridge_pid=""
  fi
  if [[ -n "$doctor_records_file" && -f "$doctor_records_file" ]]; then
    rm -f "$doctor_records_file"
  fi
}

handle_termination() {
  cleanup
  trap - EXIT
  exit 143
}

trap cleanup EXIT
trap handle_termination INT TERM HUP

if [[ -n "$bundle_dir" ]]; then
  bundle_dir="$(normalize_path "$bundle_dir")"
fi

if [[ "$mode" == "download" ]]; then
  download_release="1"
fi

if [[ "$download_release" == "1" ]]; then
  bundle_dir="$(download_release_bundle)"
fi

repo_required="0"
need_cargo="0"
need_provider_cli="0"
use_bundle_play="0"

if [[ "$execution_mode" != "headless_agent" && "$execution_mode" != "player_parity" ]]; then
  echo "error: --execution-mode must be headless_agent or player_parity" >&2
  exit 1
fi

case "$mode" in
  download)
    ;;
  doctor)
    require_cmd curl
    ;;
  play)
    if [[ -n "$bundle_dir" ]]; then
      if ! validate_bundle_dir "$bundle_dir"; then
        echo "error: invalid --bundle-dir, missing run-game.sh: $bundle_dir" >&2
        exit 1
      fi
      use_bundle_play="1"
    fi
    if [[ "$skip_agent_setup" != "1" ]]; then
      repo_required="1"
      need_provider_cli="1"
    fi
    if [[ "$reuse_bridge" != "1" ]]; then
      repo_required="1"
      need_cargo="1"
    fi
    if [[ "$use_bundle_play" != "1" ]]; then
      repo_required="1"
      need_cargo="1"
    fi
    require_cmd curl
    ;;
  smoke)
    repo_required="1"
    need_cargo="1"
    if [[ "$skip_agent_setup" != "1" ]]; then
      need_provider_cli="1"
    fi
    require_cmd curl
    ;;
  *)
    echo "unknown mode: $mode" >&2
    usage >&2
    exit 1
    ;;
esac

if [[ "$repo_required" == "1" ]]; then
  repo_root="$(discover_repo_root || true)"
  if [[ -z "$repo_root" ]]; then
    if [[ "$mode" == "play" && "$use_bundle_play" == "1" ]]; then
      echo "error: bundle is valid at $bundle_dir, but repo-backed bridge/bootstrap for '$mode' still needs the repo root" >&2
      echo "hint: pass --repo-root <path>, or reuse an already running bridge via --reuse-bridge --skip-agent-setup" >&2
    else
      echo "error: repo root is required for '$mode'; pass --repo-root <path>" >&2
    fi
    exit 1
  fi
fi

bridge_log="$(resolve_bridge_log_default "$repo_root")"
mkdir -p "$(dirname "$bridge_log")"

if [[ "$need_provider_cli" == "1" ]]; then
  require_cmd "$provider_cli_bin"
fi
if [[ "$need_cargo" == "1" ]]; then
  if ! command -v cargo >/dev/null 2>&1; then
    if [[ "$mode" == "play" ]]; then
      if [[ "$use_bundle_play" == "1" ]]; then
        echo "error: bundle is valid at $bundle_dir, but repo-backed bridge/bootstrap for '$mode' requires cargo" >&2
        echo "hint: install cargo so oasis7 can auto-start the bridge/bootstrap path, or reuse an already running bridge via --reuse-bridge --skip-agent-setup" >&2
      else
        echo "error: source-tree '$mode' requires cargo to launch oasis7_game_launcher or the repo-backed bridge/bootstrap path" >&2
        echo "hint: install cargo, or switch to a downloaded bundle and reuse an already running bridge via --bundle-dir <path> --reuse-bridge --skip-agent-setup" >&2
      fi
      exit 1
    fi
    require_cmd cargo
  fi
fi

if [[ "$mode" == "doctor" ]]; then
  run_doctor
  exit $?
fi

if [[ "$mode" == "download" ]]; then
  printf '%s\n' "$bundle_dir"
  exit 0
fi

if [[ "$skip_agent_setup" != "1" ]]; then
  wait_for_http "http://127.0.0.1:18789/health" 20 0.5
  "$repo_root/scripts/setup-provider-oasis7-runtime.sh" "$agent_id"
fi

if [[ "$reuse_bridge" != "1" ]]; then
  wait_for_http "http://127.0.0.1:18789/health" 20 0.5
  (
    cd "$repo_root"
    exec env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_provider_local_bridge -- --provider-agent "$agent_id"
  ) >"$bridge_log" 2>&1 &
  cleanup_bridge_pid="$!"
fi

wait_for_http "$base_url/v1/provider/health" 40 0.5

case "$mode" in
  play)
    if [[ "$use_bundle_play" == "1" ]]; then
      cmd=("$bundle_dir/run-game.sh"
        --scenario "$scenario"
        --with-llm
        --agent-provider-mode provider_loopback_http
        --agent-provider-url "$base_url"
        --agent-provider-connect-timeout-ms "$connect_timeout_ms"
        --agent-provider-profile "$agent_profile"
        --agent-execution-lane "$execution_mode")
    else
      viewer_static_out_dir="$repo_root/output/oasis7/viewer-static-$(date +%Y%m%d-%H%M%S)"
      resolved_viewer_static_dir="$(resolve_source_tree_viewer_static_dir "$repo_root" "$viewer_static_out_dir")"
      cd "$repo_root"
      cmd=(env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher --
        --scenario "$scenario"
        --with-llm
        --agent-provider-mode provider_loopback_http
        --agent-provider-url "$base_url"
        --agent-provider-connect-timeout-ms "$connect_timeout_ms"
        --agent-provider-profile "$agent_profile"
        --agent-execution-lane "$execution_mode"
        --viewer-static-dir "$resolved_viewer_static_dir")
    fi
    if [[ "$open_browser" != "1" ]]; then
      cmd+=(--no-open-browser)
    fi
    printf 'Running: %q ' "${cmd[@]}"
    printf '\n'
    if command -v setsid >/dev/null 2>&1; then
      setsid "${cmd[@]}" &
      cleanup_play_pid="$!"
      cleanup_play_pgid="$cleanup_play_pid"
    else
      "${cmd[@]}" &
      cleanup_play_pid="$!"
      cleanup_play_pgid=""
    fi
    set +e
    wait "$cleanup_play_pid"
    play_status=$?
    set -e
    cleanup_play_pid=""
    cleanup_play_pgid=""
    exit "$play_status"
    ;;
  smoke)
    cd "$repo_root"
    cmd=(bash scripts/provider-parity-p0.sh
      --provider-only
      --samples "$samples"
      --ticks "$ticks"
      --timeout-ms "$timeout_ms"
      --agent-provider-url "$base_url"
      --agent-provider-connect-timeout-ms "$connect_timeout_ms"
      --agent-provider-profile "$agent_profile"
      --execution-mode "$execution_mode")
    printf 'Running: %q ' "${cmd[@]}"
    printf '\n'
    "${cmd[@]}"
    ;;
esac
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
