#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-real-env-triad-snapshot.sh [options]

Capture a P2PARCH-6 real-environment triad snapshot from:
  - local observer node
  - remote ECS sequencer node
  - remote ECS storage node

Options:
  --samples <n>                    number of status samples per node (default: 4)
  --interval-secs <n>              sleep interval between samples (default: 5)
  --ssh-timeout-secs <n>           SSH connect timeout in seconds (default: 8)
  --out-dir <path>                 output root (default: .tmp/p2p_real_env_triad)
  --world-id <id>                  expected world id (default: shared-devnet-ecs-v1)

  --observer-service <name>        local observer systemd unit
                                   (default: oasis7-triad-observer.service)
  --observer-status-url <url>      local observer status endpoint
                                   (default: http://127.0.0.1:5633/v1/chain/status)
  --observer-health-url <url>      local observer health endpoint
                                   (default: http://127.0.0.1:5633/healthz)
  --observer-env-file <path>       local observer env file
                                   (default: /opt/oasis7/p2p-triad-local/config/node.env)

  --sequencer-target <user@host>   remote sequencer SSH target
                                   (default: root@39.104.204.172)
  --sequencer-service <name>       remote sequencer systemd unit
                                   (default: oasis7-triad-sequencer.service)
  --sequencer-status-url <url>     remote sequencer status endpoint
                                   (default: http://127.0.0.1:5631/v1/chain/status)
  --sequencer-health-url <url>     remote sequencer health endpoint
                                   (default: http://127.0.0.1:5631/healthz)
  --sequencer-env-file <path>      remote sequencer env file
                                   (default: /opt/oasis7/p2p-triad/config/node.env)

  --storage-target <user@host>     remote storage SSH target
                                   (default: root@39.104.205.67)
  --storage-service <name>         remote storage systemd unit
                                   (default: oasis7-triad-storage.service)
  --storage-status-url <url>       remote storage status endpoint
                                   (default: http://127.0.0.1:5632/v1/chain/status)
  --storage-health-url <url>       remote storage health endpoint
                                   (default: http://127.0.0.1:5632/healthz)
  --storage-env-file <path>        remote storage env file
                                   (default: /opt/oasis7/p2p-triad/config/node.env)

Environment:
  P2PARCH6_SEQ_SSH_PASSWORD        optional sequencer SSH password for sshpass
  P2PARCH6_STORAGE_SSH_PASSWORD    optional storage SSH password for sshpass

Notes:
  - No secrets are written into the repository or the generated summary.
  - If the password env vars are unset, the script falls back to plain SSH and
    assumes key-based auth already works.
  - Artifacts are written to:
      <out-dir>/<timestamp>/{
        config.json,
        samples.ndjson,
        summary.json,
        summary.md,
        nodes/<label>/{node.env,service_state.txt,healthz.json,status.json,samples/*}
      }
USAGE
}

ensure_positive_int() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^[0-9]+$ ]] || (( value <= 0 )); then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

run_ssh() {
  local target=$1
  local password=${2:-}
  shift 2
  local cmd=(
    ssh
    -o StrictHostKeyChecking=no
    -o ConnectTimeout="$ssh_timeout_secs"
    -o ServerAliveInterval=5
    "$target"
    "$@"
  )
  if [[ -n "$password" ]]; then
    SSHPASS="$password" sshpass -e "${cmd[@]}"
  else
    "${cmd[@]}"
  fi
}

record_env_copy() {
  local label=$1
  local mode=$2
  local source=$3
  local password=${4:-}
  local node_dir="$nodes_root/$label"
  if [[ "$mode" == "local" ]]; then
    if [[ -f "$source" ]]; then
      cp "$source" "$node_dir/node.env"
    else
      printf 'missing local env file: %s\n' "$source" > "$node_dir/node.env.error.txt"
    fi
    return 0
  fi
  if run_ssh "$source" "$password" "cat '$5'" > "$node_dir/node.env" 2>"$node_dir/node.env.stderr.log"; then
    return 0
  fi
  printf 'failed to fetch remote env file from %s\n' "$source" > "$node_dir/node.env.error.txt"
}

capture_service_state() {
  local label=$1
  local mode=$2
  local service=$3
  local target=${4:-}
  local password=${5:-}
  local node_dir="$nodes_root/$label"
  local service_state_file="$node_dir/service_state.txt"
  if [[ "$mode" == "local" ]]; then
    if systemctl is-active "$service" >"$service_state_file" 2>"$node_dir/service_state.stderr.log"; then
      return 0
    fi
    return 0
  fi
  run_ssh "$target" "$password" "systemctl is-active '$service'" >"$service_state_file" 2>"$node_dir/service_state.stderr.log" || true
}

capture_health_and_status() {
  local label=$1
  local mode=$2
  local health_url=$3
  local status_url=$4
  local sample_index=$5
  local target=${6:-}
  local password=${7:-}
  local node_dir="$nodes_root/$label"
  local sample_dir="$node_dir/samples/sample-$(printf '%03d' "$sample_index")"
  mkdir -p "$sample_dir"

  if [[ "$mode" == "local" ]]; then
    if curl -fsS "$health_url" >"$sample_dir/healthz.json" 2>"$sample_dir/healthz.stderr.log"; then
      cp "$sample_dir/healthz.json" "$node_dir/healthz.json"
    else
      printf '{"ok":false,"fetch_error":"curl_failed"}\n' >"$sample_dir/healthz.json"
    fi
    if curl -fsS "$status_url" >"$sample_dir/status.json" 2>"$sample_dir/status.stderr.log"; then
      cp "$sample_dir/status.json" "$node_dir/status.json"
    else
      printf '{"ok":false,"fetch_error":"curl_failed"}\n' >"$sample_dir/status.json"
    fi
    return 0
  fi

  if run_ssh "$target" "$password" "curl -fsS '$health_url'" >"$sample_dir/healthz.json" 2>"$sample_dir/healthz.stderr.log"; then
    cp "$sample_dir/healthz.json" "$node_dir/healthz.json"
  else
    printf '{"ok":false,"fetch_error":"ssh_or_curl_failed"}\n' >"$sample_dir/healthz.json"
  fi

  if run_ssh "$target" "$password" "curl -fsS '$status_url'" >"$sample_dir/status.json" 2>"$sample_dir/status.stderr.log"; then
    cp "$sample_dir/status.json" "$node_dir/status.json"
  else
    printf '{"ok":false,"fetch_error":"ssh_or_curl_failed"}\n' >"$sample_dir/status.json"
  fi
}

append_sample_record() {
  local label=$1
  local service=$2
  local service_state_file="$nodes_root/$label/service_state.txt"
  local sample_dir="$nodes_root/$label/samples/sample-$(printf '%03d' "$sample_index")"
  local service_state="unknown"
  if [[ -f "$service_state_file" ]]; then
    service_state=$(tr -d '\r' < "$service_state_file" | tail -n 1)
  fi

  jq -n \
    --arg label "$label" \
    --arg service "$service" \
    --arg sample_index "$sample_index" \
    --arg captured_at "$captured_at" \
    --arg expected_world_id "$world_id" \
    --arg service_state "$service_state" \
    --slurpfile health "$sample_dir/healthz.json" \
    --slurpfile status "$sample_dir/status.json" \
    '{
      sample_index: ($sample_index | tonumber),
      captured_at: $captured_at,
      label: $label,
      service: $service,
      service_state: $service_state,
      expected_world_id: $expected_world_id,
      healthz_ok: ($health[0].ok // false),
      healthz_fetch_error: ($health[0].fetch_error // null),
      status_fetch_ok: ($status[0].ok // false),
      status_fetch_error: ($status[0].fetch_error // null),
      node_id: ($status[0].node_id // null),
      world_id: ($status[0].world_id // null),
      role: ($status[0].role // null),
      running: ($status[0].running // null),
      last_error: ($status[0].last_error // null),
      reward_runtime_last_error: ($status[0].reward_runtime.last_error // null),
      consensus: {
        slot: ($status[0].consensus.slot // null),
        latest_height: ($status[0].consensus.latest_height // null),
        committed_height: ($status[0].consensus.committed_height // null),
        network_committed_height: ($status[0].consensus.network_committed_height // null),
        known_peer_heads: ($status[0].consensus.known_peer_heads // null),
        missed_slot_count: ($status[0].consensus.missed_slot_count // null),
        missed_tick_count: ($status[0].consensus.missed_tick_count // null)
      }
    }' >> "$samples_ndjson"
}

samples=4
interval_secs=5
ssh_timeout_secs=8
out_root=".tmp/p2p_real_env_triad"
world_id="shared-devnet-ecs-v1"

observer_service="oasis7-triad-observer.service"
observer_status_url="http://127.0.0.1:5633/v1/chain/status"
observer_health_url="http://127.0.0.1:5633/healthz"
observer_env_file="/opt/oasis7/p2p-triad-local/config/node.env"

sequencer_target="root@39.104.204.172"
sequencer_service="oasis7-triad-sequencer.service"
sequencer_status_url="http://127.0.0.1:5631/v1/chain/status"
sequencer_health_url="http://127.0.0.1:5631/healthz"
sequencer_env_file="/opt/oasis7/p2p-triad/config/node.env"

storage_target="root@39.104.205.67"
storage_service="oasis7-triad-storage.service"
storage_status_url="http://127.0.0.1:5632/v1/chain/status"
storage_health_url="http://127.0.0.1:5632/healthz"
storage_env_file="/opt/oasis7/p2p-triad/config/node.env"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --samples)
      samples=${2:-}
      shift 2
      ;;
    --interval-secs)
      interval_secs=${2:-}
      shift 2
      ;;
    --ssh-timeout-secs)
      ssh_timeout_secs=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --world-id)
      world_id=${2:-}
      shift 2
      ;;
    --observer-service)
      observer_service=${2:-}
      shift 2
      ;;
    --observer-status-url)
      observer_status_url=${2:-}
      shift 2
      ;;
    --observer-health-url)
      observer_health_url=${2:-}
      shift 2
      ;;
    --observer-env-file)
      observer_env_file=${2:-}
      shift 2
      ;;
    --sequencer-target)
      sequencer_target=${2:-}
      shift 2
      ;;
    --sequencer-service)
      sequencer_service=${2:-}
      shift 2
      ;;
    --sequencer-status-url)
      sequencer_status_url=${2:-}
      shift 2
      ;;
    --sequencer-health-url)
      sequencer_health_url=${2:-}
      shift 2
      ;;
    --sequencer-env-file)
      sequencer_env_file=${2:-}
      shift 2
      ;;
    --storage-target)
      storage_target=${2:-}
      shift 2
      ;;
    --storage-service)
      storage_service=${2:-}
      shift 2
      ;;
    --storage-status-url)
      storage_status_url=${2:-}
      shift 2
      ;;
    --storage-health-url)
      storage_health_url=${2:-}
      shift 2
      ;;
    --storage-env-file)
      storage_env_file=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

ensure_positive_int --samples "$samples"
ensure_positive_int --interval-secs "$interval_secs"
ensure_positive_int --ssh-timeout-secs "$ssh_timeout_secs"

run_id=$(date +"%Y%m%d-%H%M%S")
run_dir="$out_root/$run_id"
nodes_root="$run_dir/nodes"
samples_ndjson="$run_dir/samples.ndjson"
summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"
config_json="$run_dir/config.json"

mkdir -p \
  "$nodes_root/observer_local/samples" \
  "$nodes_root/sequencer_ecs/samples" \
  "$nodes_root/storage_ecs/samples"
: > "$samples_ndjson"

seq_password=${P2PARCH6_SEQ_SSH_PASSWORD:-}
storage_password=${P2PARCH6_STORAGE_SSH_PASSWORD:-}

jq -n \
  --arg run_id "$run_id" \
  --arg run_dir "$run_dir" \
  --arg world_id "$world_id" \
  --arg observer_service "$observer_service" \
  --arg observer_status_url "$observer_status_url" \
  --arg observer_health_url "$observer_health_url" \
  --arg observer_env_file "$observer_env_file" \
  --arg sequencer_target "$sequencer_target" \
  --arg sequencer_service "$sequencer_service" \
  --arg sequencer_status_url "$sequencer_status_url" \
  --arg sequencer_health_url "$sequencer_health_url" \
  --arg sequencer_env_file "$sequencer_env_file" \
  --arg storage_target "$storage_target" \
  --arg storage_service "$storage_service" \
  --arg storage_status_url "$storage_status_url" \
  --arg storage_health_url "$storage_health_url" \
  --arg storage_env_file "$storage_env_file" \
  --argjson samples "$samples" \
  --argjson interval_secs "$interval_secs" \
  --argjson ssh_timeout_secs "$ssh_timeout_secs" \
  '{
    run_id: $run_id,
    run_dir: $run_dir,
    world_id: $world_id,
    samples: $samples,
    interval_secs: $interval_secs,
    ssh_timeout_secs: $ssh_timeout_secs,
    nodes: {
      observer_local: {
        mode: "local",
        service: $observer_service,
        status_url: $observer_status_url,
        health_url: $observer_health_url,
        env_file: $observer_env_file
      },
      sequencer_ecs: {
        mode: "remote",
        target: $sequencer_target,
        service: $sequencer_service,
        status_url: $sequencer_status_url,
        health_url: $sequencer_health_url,
        env_file: $sequencer_env_file
      },
      storage_ecs: {
        mode: "remote",
        target: $storage_target,
        service: $storage_service,
        status_url: $storage_status_url,
        health_url: $storage_health_url,
        env_file: $storage_env_file
      }
    }
  }' > "$config_json"

record_env_copy observer_local local "$observer_env_file"
record_env_copy sequencer_ecs remote "$sequencer_target" "$seq_password" "$sequencer_env_file"
record_env_copy storage_ecs remote "$storage_target" "$storage_password" "$storage_env_file"

capture_service_state observer_local local "$observer_service"
capture_service_state sequencer_ecs remote "$sequencer_service" "$sequencer_target" "$seq_password"
capture_service_state storage_ecs remote "$storage_service" "$storage_target" "$storage_password"

started_at=$(date -Iseconds)

for ((sample_index = 1; sample_index <= samples; sample_index++)); do
  captured_at=$(date -Iseconds)
  echo "sample $sample_index/$samples @ $captured_at"

  capture_health_and_status observer_local local "$observer_health_url" "$observer_status_url" "$sample_index"
  append_sample_record observer_local "$observer_service"

  capture_health_and_status sequencer_ecs remote "$sequencer_health_url" "$sequencer_status_url" "$sample_index" "$sequencer_target" "$seq_password"
  append_sample_record sequencer_ecs "$sequencer_service"

  capture_health_and_status storage_ecs remote "$storage_health_url" "$storage_status_url" "$sample_index" "$storage_target" "$storage_password"
  append_sample_record storage_ecs "$storage_service"

  if (( sample_index < samples )); then
    sleep "$interval_secs"
  fi
done

ended_at=$(date -Iseconds)

jq -s \
  --arg started_at "$started_at" \
  --arg ended_at "$ended_at" \
  --arg run_id "$run_id" \
  --arg run_dir "$run_dir" \
  --arg world_id "$world_id" \
  '
  def heights_for($label):
    map(select(.label == $label) | .consensus.committed_height // 0);
  def values_for($label; $path):
    map(select(.label == $label) | getpath($path));
  def first_or_zero($arr):
    if ($arr | length) == 0 then 0 else ($arr[0] // 0) end;
  def last_or_zero($arr):
    if ($arr | length) == 0 then 0 else ($arr[-1] // 0) end;
  def max_or_zero($arr):
    if ($arr | length) == 0 then 0 else ($arr | map(. // 0) | max) end;
  def min_or_zero($arr):
    if ($arr | length) == 0 then 0 else ($arr | map(. // 0) | min) end;
  def node_summary($label):
    {
      label: $label,
      sample_count: (map(select(.label == $label)) | length),
      service_states: (map(select(.label == $label) | .service_state) | unique),
      healthz_all_ok: (map(select(.label == $label) | .healthz_ok) | all(. == true)),
      status_fetch_all_ok: (map(select(.label == $label) | .status_fetch_ok) | all(. == true)),
      world_ids: (map(select(.label == $label) | .world_id) | unique),
      node_ids: (map(select(.label == $label) | .node_id) | unique),
      roles: (map(select(.label == $label) | .role) | unique),
      last_errors: (map(select(.label == $label) | .last_error) | unique),
      heights: {
        first_committed_height: first_or_zero(heights_for($label)),
        last_committed_height: last_or_zero(heights_for($label)),
        max_committed_height: max_or_zero(heights_for($label)),
        min_committed_height: min_or_zero(heights_for($label))
      },
      network: {
        first_network_committed_height: first_or_zero(values_for($label; ["consensus","network_committed_height"])),
        last_network_committed_height: last_or_zero(values_for($label; ["consensus","network_committed_height"])),
        max_network_committed_height: max_or_zero(values_for($label; ["consensus","network_committed_height"]))
      },
      peers: {
        min_known_peer_heads: min_or_zero(values_for($label; ["consensus","known_peer_heads"])),
        max_known_peer_heads: max_or_zero(values_for($label; ["consensus","known_peer_heads"]))
      }
    };
  . as $samples
  | {
      run_id: $run_id,
      run_dir: $run_dir,
      world_id: $world_id,
      started_at: $started_at,
      ended_at: $ended_at,
      totals: {
        sample_record_count: length,
        node_count: (map(.label) | unique | length)
      },
      nodes: {
        observer_local: node_summary("observer_local"),
        sequencer_ecs: node_summary("sequencer_ecs"),
        storage_ecs: node_summary("storage_ecs")
      },
      samples: $samples
    }
  | .analysis = {
      cloud_pair_service_healthy: (
        (.nodes.sequencer_ecs.healthz_all_ok == true)
        and (.nodes.storage_ecs.healthz_all_ok == true)
        and (.nodes.sequencer_ecs.status_fetch_all_ok == true)
        and (.nodes.storage_ecs.status_fetch_all_ok == true)
        and ((.nodes.sequencer_ecs.service_states | index("active")) != null)
        and ((.nodes.storage_ecs.service_states | index("active")) != null)
      ),
      sequencer_chain_visible: (.nodes.sequencer_ecs.heights.max_committed_height > 0),
      storage_chain_visible: (.nodes.storage_ecs.heights.max_committed_height > 0),
      cloud_pair_chain_visible: (
        (.nodes.sequencer_ecs.heights.max_committed_height > 0)
        and (.nodes.storage_ecs.heights.max_committed_height > 0)
      ),
      cloud_pair_progress_signal_present: (
        (.nodes.sequencer_ecs.heights.last_committed_height > .nodes.sequencer_ecs.heights.first_committed_height)
        or (.nodes.storage_ecs.heights.last_committed_height > .nodes.storage_ecs.heights.first_committed_height)
      ),
      sequencer_execution_stale_height: (
        (.nodes.sequencer_ecs.last_errors | any(
          . != null and (. | test("execution driver received stale height"))
        ))
      ),
      observer_service_healthy: (
        (.nodes.observer_local.healthz_all_ok == true)
        and (.nodes.observer_local.status_fetch_all_ok == true)
        and ((.nodes.observer_local.service_states | index("active")) != null)
      ),
      observer_peer_visibility_ok: (.nodes.observer_local.peers.max_known_peer_heads > 0),
      observer_network_commit_visible: (.nodes.observer_local.network.max_network_committed_height > 0),
      observer_committed_height_progressing: (.nodes.observer_local.heights.last_committed_height > .nodes.observer_local.heights.first_committed_height)
    }
  | .failure_signatures = (
      []
      + (if .analysis.cloud_pair_service_healthy then [] else ["cloud_pair_service_unhealthy"] end)
      + (
          if .analysis.cloud_pair_chain_visible then []
          elif ((.analysis.sequencer_chain_visible | not) and (.analysis.storage_chain_visible | not))
          then ["cloud_pair_chain_not_visible"]
          else []
          end
        )
      + (if .analysis.sequencer_chain_visible then [] else ["sequencer_committed_height_zero"] end)
      + (if .analysis.storage_chain_visible then [] else ["storage_committed_height_zero"] end)
      + (if .analysis.cloud_pair_progress_signal_present then [] else ["cloud_pair_no_recent_progress_signal"] end)
      + (if .analysis.sequencer_execution_stale_height then ["sequencer_execution_stale_height"] else [] end)
      + (if .analysis.observer_service_healthy then [] else ["observer_service_unhealthy"] end)
      + (if .analysis.observer_peer_visibility_ok then [] else ["observer_known_peer_heads_zero"] end)
      + (if .analysis.observer_network_commit_visible then [] else ["observer_network_committed_height_zero"] end)
      + (if .analysis.observer_committed_height_progressing then [] else ["observer_committed_height_not_advancing"] end)
    )
  | .claim_status = (
      if .analysis.cloud_pair_service_healthy
         and .analysis.cloud_pair_chain_visible
         and .analysis.cloud_pair_progress_signal_present
         and .analysis.observer_service_healthy
         and .analysis.observer_peer_visibility_ok
         and .analysis.observer_network_commit_visible
         and .analysis.observer_committed_height_progressing
      then "pass_candidate"
      elif .analysis.cloud_pair_service_healthy
           and .analysis.cloud_pair_chain_visible
           and .analysis.cloud_pair_progress_signal_present
      then "partial_with_observer_blocker"
      else "blocked"
      end
    )
  ' "$samples_ndjson" > "$summary_json"

{
  echo "# P2P Real-Environment Triad Snapshot"
  echo
  echo "- run_id: \`$run_id\`"
  echo "- started_at: \`$started_at\`"
  echo "- ended_at: \`$ended_at\`"
  echo "- world_id: \`$world_id\`"
  echo "- claim_status: \`$(jq -r '.claim_status' "$summary_json")\`"
  echo "- failure_signatures: \`$(jq -r '.failure_signatures | if length == 0 then "(none)" else join(", ") end' "$summary_json")\`"
  echo
  echo "## Node Summary"
  for label in observer_local sequencer_ecs storage_ecs; do
    echo "### \`$label\`"
    echo "- service_states: \`$(jq -r --arg label "$label" '.nodes[$label].service_states | join(", ")' "$summary_json")\`"
    echo "- healthz_all_ok: \`$(jq -r --arg label "$label" '.nodes[$label].healthz_all_ok' "$summary_json")\`"
    echo "- status_fetch_all_ok: \`$(jq -r --arg label "$label" '.nodes[$label].status_fetch_all_ok' "$summary_json")\`"
    echo "- node_ids: \`$(jq -r --arg label "$label" '.nodes[$label].node_ids | join(", ")' "$summary_json")\`"
    echo "- roles: \`$(jq -r --arg label "$label" '.nodes[$label].roles | join(", ")' "$summary_json")\`"
    echo "- last_errors: \`$(jq -r --arg label "$label" '.nodes[$label].last_errors | map(select(. != null)) | if length == 0 then "(none)" else join(" | ") end' "$summary_json")\`"
    echo "- committed_height: \`$(jq -r --arg label "$label" '.nodes[$label].heights.first_committed_height' "$summary_json") -> $(jq -r --arg label "$label" '.nodes[$label].heights.last_committed_height' "$summary_json")\`"
    echo "- network_committed_height: \`$(jq -r --arg label "$label" '.nodes[$label].network.first_network_committed_height' "$summary_json") -> $(jq -r --arg label "$label" '.nodes[$label].network.last_network_committed_height' "$summary_json")\`"
    echo "- known_peer_heads: \`$(jq -r --arg label "$label" '.nodes[$label].peers.min_known_peer_heads' "$summary_json") -> $(jq -r --arg label "$label" '.nodes[$label].peers.max_known_peer_heads' "$summary_json")\`"
  done
  echo
  echo "## Artifacts"
  echo "- config_json: \`$config_json\`"
  echo "- samples_ndjson: \`$samples_ndjson\`"
  echo "- summary_json: \`$summary_json\`"
} > "$summary_md"

echo "p2p real-env triad snapshot complete"
echo "  run_dir: $run_dir"
echo "  summary_json: $summary_json"
echo "  summary_md: $summary_md"
