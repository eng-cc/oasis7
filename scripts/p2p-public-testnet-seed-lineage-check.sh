#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-seed-lineage-check.sh \
    --lhs-root <node-distfs-root> \
    --rhs-root <node-distfs-root> \
    [--sample-heights <csv>]

Description:
  Compare two node-distfs replication roots before reusing one seed against
  another live/shared-devnet node. The report prints:
    - node_pos_state summary
    - replication hot/cold coverage
    - hot commit file / blob counts
    - sampled height lineage hashes from hot files or cold-index entries

  A sampled height is considered divergent when the two roots resolve to
  different replication content hashes for the same height.
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

lhs_root=""
rhs_root=""
sample_heights="1,366,617,733"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --lhs-root)
      lhs_root=${2:-}
      shift 2
      ;;
    --rhs-root)
      rhs_root=${2:-}
      shift 2
      ;;
    --sample-heights)
      sample_heights=${2:-}
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
require_command sha256sum
[[ -n "$lhs_root" ]] || die "--lhs-root is required"
[[ -n "$rhs_root" ]] || die "--rhs-root is required"
require_dir "$lhs_root"
require_dir "$rhs_root"

node_pos_state_path() {
  local root=$1
  printf '%s/node_pos_state.json' "$root"
}

hot_dir_path() {
  local root=$1
  printf '%s/replication_commit_messages' "$root"
}

cold_index_path() {
  local root=$1
  if [[ -f "$root/replication_commit_messages.cold-index/index.json" ]]; then
    printf '%s/replication_commit_messages.cold-index/index.json' "$root"
  elif [[ -f "$root/replication_commit_messages_cold_index.json" ]]; then
    printf '%s/replication_commit_messages_cold_index.json' "$root"
  else
    return 1
  fi
}

blob_dir_path() {
  local root=$1
  printf '%s/store/blobs' "$root"
}

height_filename() {
  local height=$1
  printf '%020d.json' "$height"
}

hot_commit_path() {
  local root=$1
  local height=$2
  printf '%s/%s' "$(hot_dir_path "$root")" "$(height_filename "$height")"
}

print_root_summary() {
  local label=$1
  local root=$2
  local pos_path
  local cold_index=""
  local hot_count=0
  local blob_count=0

  pos_path=$(node_pos_state_path "$root")
  require_file "$pos_path"

  if [[ -d "$(hot_dir_path "$root")" ]]; then
    hot_count=$(find "$(hot_dir_path "$root")" -maxdepth 1 -type f | wc -l | tr -d ' ')
  fi
  if [[ -d "$(blob_dir_path "$root")" ]]; then
    blob_count=$(find "$(blob_dir_path "$root")" -maxdepth 1 -type f | wc -l | tr -d ' ')
  fi
  cold_index=$(cold_index_path "$root" || true)

  printf '=== %s\n' "$label"
  printf 'root=%s\n' "$root"
  jq -r '"pos committed=\(.committed_height) network=\(.network_committed_height) exec=\(.last_execution_height) root=\(.last_execution_state_root)"' "$pos_path"
  printf 'hot_commit_files=%s\n' "$hot_count"
  printf 'blob_files=%s\n' "$blob_count"
  if [[ -n "$cold_index" ]]; then
    jq -r '"cold_index hot_range=\(.hot_range.from_key // "null")..\(.hot_range.to_key // "null") cold_anchor=\(.cold_range_anchor.from_key // "null")..\(.cold_range_anchor.to_key // "null") entry_count=\(.cold_range_anchor.entry_count // "null")"' "$cold_index"
  else
    printf 'cold_index=missing\n'
  fi
}

content_hash_for_height() {
  local root=$1
  local height=$2
  local hot_path
  local cold_index=""

  hot_path=$(hot_commit_path "$root" "$height")
  if [[ -f "$hot_path" ]]; then
    jq -r '.record.content_hash' "$hot_path"
    return 0
  fi

  cold_index=$(cold_index_path "$root" || true)
  if [[ -n "$cold_index" ]]; then
    jq -r --arg h "$height" '.by_height[$h].content_hash // empty' "$cold_index"
    return 0
  fi

  printf ''
}

content_source_for_height() {
  local root=$1
  local height=$2
  local hot_path

  hot_path=$(hot_commit_path "$root" "$height")
  if [[ -f "$hot_path" ]]; then
    printf 'hot'
    return 0
  fi
  if [[ -n "$(cold_index_path "$root" || true)" ]]; then
    printf 'cold'
    return 0
  fi
  printf 'missing'
}

file_sha_for_height() {
  local root=$1
  local height=$2
  local hot_path

  hot_path=$(hot_commit_path "$root" "$height")
  if [[ -f "$hot_path" ]]; then
    sha256sum "$hot_path" | awk '{print $1}'
  fi
}

print_root_summary "lhs" "$lhs_root"
printf '\n'
print_root_summary "rhs" "$rhs_root"
printf '\n'

IFS=, read -r -a sample_height_list <<< "$sample_heights"
divergent_count=0
unknown_count=0

printf '=== sample_heights\n'
for raw_height in "${sample_height_list[@]}"; do
  height=$(printf '%s' "$raw_height" | tr -d '[:space:]')
  [[ -n "$height" ]] || continue

  lhs_hash=$(content_hash_for_height "$lhs_root" "$height")
  rhs_hash=$(content_hash_for_height "$rhs_root" "$height")
  lhs_source=$(content_source_for_height "$lhs_root" "$height")
  rhs_source=$(content_source_for_height "$rhs_root" "$height")
  lhs_file_sha=$(file_sha_for_height "$lhs_root" "$height")
  rhs_file_sha=$(file_sha_for_height "$rhs_root" "$height")
  verdict="unknown"

  if [[ -n "$lhs_hash" && -n "$rhs_hash" ]]; then
    if [[ "$lhs_source" == "$rhs_source" ]]; then
      if [[ "$lhs_hash" == "$rhs_hash" ]]; then
        verdict="match"
      else
        verdict="divergent"
        divergent_count=$(( divergent_count + 1 ))
      fi
    else
      verdict="representation-mismatch"
      unknown_count=$(( unknown_count + 1 ))
    fi
  else
    unknown_count=$(( unknown_count + 1 ))
  fi

  printf 'height=%s verdict=%s lhs_source=%s rhs_source=%s\n' "$height" "$verdict" "$lhs_source" "$rhs_source"
  printf '  lhs_content_hash=%s\n' "${lhs_hash:-missing}"
  printf '  rhs_content_hash=%s\n' "${rhs_hash:-missing}"
  if [[ -n "$lhs_file_sha" || -n "$rhs_file_sha" ]]; then
    printf '  lhs_file_sha256=%s\n' "${lhs_file_sha:-missing}"
    printf '  rhs_file_sha256=%s\n' "${rhs_file_sha:-missing}"
  fi
done

printf '\n'
if (( divergent_count > 0 )); then
  printf 'lineage_verdict=divergent\n'
elif (( unknown_count > 0 )); then
  printf 'lineage_verdict=inconclusive\n'
else
  printf 'lineage_verdict=match_on_sample\n'
fi
