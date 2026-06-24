#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --node-root <path> \
    --bundle-tar <oasis7-linux-x64-bundle.tar.gz> \
    --package-version <version> \
    --commit <sha> \
    --run-id <github-actions-run-id> \
    [--artifact-ref <ref>] \
    [--systemd-service <name>] \
    [--release-retention-count <count>] \
    [--restart-service] \
    [--post-restart-status-url <url>] \
    [--post-restart-timeout-secs <secs>]

Description:
  Upgrade an installed public testnet Linux node from a CI package bundle.
  The script extracts the bundle into <node-root>/releases/<package-version>,
  repoints <node-root>/current, and rewrites the node-local governed bootstrap
  bundle runtime_build hash to the installed runtime binary. This keeps the
  network-tier runtime drift guard aligned with the deployed artifact.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_non_empty() {
  local flag=$1
  local value=$2
  [[ -n "$value" ]] || die "missing required option: $flag"
}

abs_path() {
  python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).expanduser().resolve())' "$1"
}

parse_node_id() {
  local start_script=$1
  python3 - "$start_script" <<'PY'
from __future__ import annotations

import shlex
import sys
from pathlib import Path

path = Path(sys.argv[1])
if not path.exists():
    raise SystemExit(0)

try:
    tokens = shlex.split(path.read_text(encoding="utf-8"))
except Exception:
    raise SystemExit(0)

for index, token in enumerate(tokens):
    if token == "--node-id" and index + 1 < len(tokens):
        print(tokens[index + 1])
        break
PY
}

cleanup_upgrade_lock() {
  if [[ -n "${upgrade_lock_dir:-}" && -d "$upgrade_lock_dir" ]]; then
    rmdir "$upgrade_lock_dir" 2>/dev/null || true
  fi
}

assert_no_node_processes() {
  local root=$1
  local matches
  matches="$(
    while IFS= read -r line; do
      if [[ "$line" =~ ^[[:space:]]*([0-9]+)[[:space:]]+([0-9]+)[[:space:]]+(.*)$ ]]; then
        args="${BASH_REMATCH[3]}"
        if [[ "$args" == *"$root"/* ]] \
          && { [[ "$args" == *"/oasis7_chain_runtime"* ]] || [[ "$args" == *"/start-node.sh"* ]]; }; then
          printf '%s\n' "$line"
        fi
      fi
    done < <(ps -eo pid=,ppid=,args=) || true
  )"
  if [[ -n "$matches" ]]; then
    printf '%s\n' "$matches" >&2
    die "node-root still has running oasis7 process after stop: $root"
  fi
}

ensure_governed_bootstrap_bundle_exists() {
  local root=$1
  local first_bundle
  first_bundle="$(
    find "$root/config" -type f \
      -name "public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
      -print -quit 2>/dev/null || true
  )"
  [[ -n "$first_bundle" ]] || die "no governed bootstrap bundle found under $root/config"
}

migrate_legacy_replication_root() {
  local root=$1
  local node_id=$2
  [[ -n "$node_id" ]] || return 0

  local legacy_root="$root/output/node-distfs/$node_id"
  local replication_root="$root/data/replication-root"
  [[ -d "$legacy_root" ]] || return 0
  [[ -d "$replication_root" ]] || mkdir -p "$replication_root"

  python3 - "$legacy_root" "$replication_root" "$node_id" <<'PY'
from __future__ import annotations

import shutil
import sys
from pathlib import Path

legacy_root = Path(sys.argv[1])
replication_root = Path(sys.argv[2])
node_id = sys.argv[3]

copied_commits = 0
copied_blobs = 0
copied_metadata = 0

def copy_missing_tree(src_dir: Path, dst_dir: Path, pattern: str) -> int:
    copied = 0
    if not src_dir.is_dir():
        return copied
    dst_dir.mkdir(parents=True, exist_ok=True)
    for src in src_dir.glob(pattern):
        if not src.is_file():
            continue
        dst = dst_dir / src.name
        if dst.exists():
            continue
        shutil.copy2(src, dst)
        copied += 1
    return copied

copied_commits += copy_missing_tree(
    legacy_root / "replication_commit_messages",
    replication_root / "replication_commit_messages",
    "*.json",
)
copied_blobs += copy_missing_tree(
    legacy_root / "store" / "blobs",
    replication_root / "store" / "blobs",
    "*.blob",
)

for name in [
    "replication_commit_messages_cold_index.json",
    "replication_remote_guards.json",
    f"replication_writer_state_{node_id}.json",
]:
    src = legacy_root / name
    dst = replication_root / name
    if src.is_file() and not dst.exists():
        shutil.copy2(src, dst)
        copied_metadata += 1

print(
    "legacy_replication_migration="
    f"node_id={node_id} commits={copied_commits} blobs={copied_blobs} metadata={copied_metadata}"
)
PY
}

prune_old_releases() {
  local releases_dir=$1
  local current_path=$2
  local retention_count=$3
  shift 3

  python3 - "$releases_dir" "$current_path" "$retention_count" "$@" <<'PY'
from __future__ import annotations

import shutil
import sys
from pathlib import Path

releases_dir = Path(sys.argv[1]).resolve()
current_path = Path(sys.argv[2]).resolve(strict=False)
retention_count = int(sys.argv[3])
extra_keep_paths = [Path(value).resolve(strict=False) for value in sys.argv[4:] if value]

if retention_count < 0:
    raise SystemExit("release retention count must be non-negative")
if not releases_dir.is_dir():
    raise SystemExit(0)

entries = sorted(
    (
        path
        for path in releases_dir.iterdir()
        if path.is_dir() and not path.name.startswith(".")
    ),
    key=lambda path: (path.stat().st_mtime_ns, path.name),
    reverse=True,
)

keep = {current_path}
keep.update(extra_keep_paths)
for path in entries[:retention_count]:
    keep.add(path.resolve(strict=False))

for path in entries:
    resolved = path.resolve(strict=False)
    if resolved in keep:
        print(f"retained_release={path}")
        continue
    try:
        resolved.relative_to(releases_dir)
    except ValueError:
        print(f"skip_release_outside_root={path}", file=sys.stderr)
        continue
    shutil.rmtree(path)
    print(f"pruned_release={path}")
PY
}

node_root=""
bundle_tar=""
package_version=""
commit=""
run_id=""
artifact_ref=""
systemd_service=""
release_retention_count=3
restart_service=0
post_restart_status_url=""
post_restart_timeout_secs=60

while [[ $# -gt 0 ]]; do
  case "$1" in
    --node-root)
      node_root=${2:-}
      shift 2
      ;;
    --bundle-tar)
      bundle_tar=${2:-}
      shift 2
      ;;
    --package-version)
      package_version=${2:-}
      shift 2
      ;;
    --commit)
      commit=${2:-}
      shift 2
      ;;
    --run-id)
      run_id=${2:-}
      shift 2
      ;;
    --artifact-ref)
      artifact_ref=${2:-}
      shift 2
      ;;
    --systemd-service)
      systemd_service=${2:-}
      shift 2
      ;;
    --release-retention-count)
      release_retention_count=${2:-}
      shift 2
      ;;
    --restart-service)
      restart_service=1
      shift
      ;;
    --post-restart-status-url)
      post_restart_status_url=${2:-}
      shift 2
      ;;
    --post-restart-timeout-secs)
      post_restart_timeout_secs=${2:-}
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

require_non_empty "--node-root" "$node_root"
require_non_empty "--bundle-tar" "$bundle_tar"
require_non_empty "--package-version" "$package_version"
require_non_empty "--commit" "$commit"
require_non_empty "--run-id" "$run_id"
[[ -f "$bundle_tar" ]] || die "missing bundle tar: $bundle_tar"
if [[ "$restart_service" -eq 1 ]]; then
  require_non_empty "--systemd-service" "$systemd_service"
fi
if [[ -n "$post_restart_status_url" && "$restart_service" -ne 1 ]]; then
  die "--post-restart-status-url requires --restart-service"
fi
if [[ ! "$post_restart_timeout_secs" =~ ^[0-9]+$ || "$post_restart_timeout_secs" -le 0 ]]; then
  die "--post-restart-timeout-secs must be a positive integer"
fi
if [[ ! "$release_retention_count" =~ ^[0-9]+$ ]]; then
  die "--release-retention-count must be a non-negative integer"
fi

node_root=$(abs_path "$node_root")
bundle_tar=$(abs_path "$bundle_tar")
artifact_ref=${artifact_ref:-"oasis7-linux-x64-bundle.tar.gz!/bin/oasis7_chain_runtime"}
node_id=$(parse_node_id "$node_root/bin/start-node.sh")
upgrade_lock_dir="$node_root/.package-upgrade.lock"
if ! mkdir "$upgrade_lock_dir" 2>/dev/null; then
  die "another package upgrade is already running for $node_root"
fi
trap cleanup_upgrade_lock EXIT

release_dir="$node_root/releases/$package_version"
tmp_dir="$node_root/releases/.${package_version}.tmp.$$"
backup_suffix="pre-${package_version//[^A-Za-z0-9_.-]/_}-$(date -u +%Y%m%dT%H%M%SZ)"

mkdir -p "$node_root/releases"
rm -rf "$tmp_dir"
mkdir -p "$tmp_dir"
tar -xzf "$bundle_tar" -C "$tmp_dir"
bundle_root="$tmp_dir/oasis7-linux-x64"
runtime_bin="$bundle_root/bin/oasis7_chain_runtime"
[[ -x "$runtime_bin" ]] || die "bundle missing executable runtime: $runtime_bin"
ensure_governed_bootstrap_bundle_exists "$node_root"

if [[ "$restart_service" -eq 1 ]]; then
  systemctl daemon-reload
  systemctl stop "$systemd_service"
  sleep 2
  assert_no_node_processes "$node_root"
fi

python3 - "$node_root" "$bundle_root" "$package_version" "$commit" "$run_id" "$artifact_ref" <<'PY'
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

node_root = Path(sys.argv[1])
bundle_root = Path(sys.argv[2])
package_version = sys.argv[3]
commit = sys.argv[4]
run_id = sys.argv[5]
artifact_ref = sys.argv[6]
runtime_bin = bundle_root / "bin" / "oasis7_chain_runtime"

digest = hashlib.sha256()
with runtime_bin.open("rb") as fh:
    for chunk in iter(lambda: fh.read(1024 * 1024), b""):
        digest.update(chunk)
runtime_sha = digest.hexdigest()
runtime_size = runtime_bin.stat().st_size
updated_by = (
    "p2p-public-testnet-package-node-upgrade "
    f"{package_version} (run {run_id}, commit {commit})"
)

bundle_paths = sorted((node_root / "config").rglob(
    "public-testnet-governed-bootstrap-bundle-2026-06-06.json"
))
if not bundle_paths:
    raise SystemExit(
        f"no governed bootstrap bundle found under {node_root / 'config'}"
    )

for bundle_path in bundle_paths:
    data = json.loads(bundle_path.read_text(encoding="utf-8"))
    runtime = data.setdefault("runtime_build", {})
    runtime["git_commit"] = commit
    runtime["kind"] = "file"
    installed_runtime = node_root / "current" / "bin" / "oasis7_chain_runtime"
    runtime["path"] = str(installed_runtime)
    runtime["resolved_path"] = str(installed_runtime)
    runtime["ref"] = artifact_ref
    runtime["sha256"] = runtime_sha
    runtime["size_bytes"] = runtime_size
    runtime["updated_by"] = updated_by
    data["git_commit"] = commit
    data["updated_by"] = updated_by
    bundle_path.write_text(
        json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

(node_root / "CURRENT_VERSION").write_text(package_version + "\n", encoding="utf-8")
(node_root / "DEPLOYED_BUILDINFO").write_text(
    "\n".join(
        [
            "workflow=Testnet Packages",
            f"run_id={run_id}",
            f"repository=eng-cc/oasis7",
            f"commit={commit}",
            f"package_version={package_version}",
            "platform=linux-x64",
            f"runtime_sha256={runtime_sha}",
            f"runtime_size={runtime_size}",
            "",
        ]
    ),
    encoding="utf-8",
)
print(f"runtime_sha256={runtime_sha}")
print(f"runtime_size={runtime_size}")
print(f"updated_bundle_count={len(bundle_paths)}")
PY

rm -rf "$release_dir"
mv "$bundle_root" "$release_dir"
rm -rf "$tmp_dir"

current_path="$node_root/current"
previous_current_path=""
if [[ -L "$current_path" || -e "$current_path" ]]; then
  previous_current_path="$(readlink -f "$current_path" || true)"
  if [[ -n "$previous_current_path" ]]; then
    printf '%s\n' "$previous_current_path" >"$node_root/last-$backup_suffix.txt"
  fi
  if [[ -d "$current_path" && ! -L "$current_path" ]]; then
    mv "$current_path" "$node_root/current-$backup_suffix.dir"
  else
    rm -f "$current_path"
  fi
fi
ln -s "$release_dir" "$current_path"

migrate_legacy_replication_root "$node_root" "$node_id"
prune_old_releases "$node_root/releases" "$current_path" "$release_retention_count" "$previous_current_path"

if [[ "$restart_service" -eq 1 ]]; then
  systemctl daemon-reload
  systemctl start "$systemd_service"
  sleep 3
  systemctl is-active --quiet "$systemd_service"
  systemctl --no-pager --full status "$systemd_service" | sed -n '1,18p'
  if [[ -n "$post_restart_status_url" ]]; then
    deadline=$((SECONDS + post_restart_timeout_secs))
    last_status=""
    while (( SECONDS < deadline )); do
      if status_json="$(curl -fsS --max-time 5 "$post_restart_status_url" 2>/dev/null)"; then
        last_status="$status_json"
        if jq -e '
          .running == true
          and (.last_error == null or .last_error == "null")
          and (.readiness.status // null) == "ready"
          and (.consensus.storage_challenge_network_degraded_height // null) == null
          and ((.observability.storage_challenge_network_degraded // false) | not)
        ' >/dev/null <<<"$status_json"; then
          echo "post_restart_readiness=ready"
          break
        fi
      fi
      sleep 3
    done
    if [[ -z "$last_status" ]] || ! jq -e '
      .running == true
      and (.last_error == null or .last_error == "null")
      and (.readiness.status // null) == "ready"
      and (.consensus.storage_challenge_network_degraded_height // null) == null
      and ((.observability.storage_challenge_network_degraded // false) | not)
    ' >/dev/null <<<"$last_status"; then
      echo "error: post-restart status did not become ready before timeout" >&2
      if [[ -n "$last_status" ]]; then
        jq -S '{running,last_error,readiness,consensus:{committed_height:.consensus.committed_height,storage_challenge_network_degraded_height:.consensus.storage_challenge_network_degraded_height,storage_challenge_network_degraded_reason:.consensus.storage_challenge_network_degraded_reason},observability:{storage_challenge_network_degraded:.observability.storage_challenge_network_degraded}}' <<<"$last_status" >&2 || true
      fi
      exit 1
    fi
  fi
fi

echo "upgraded $node_root to $package_version"
