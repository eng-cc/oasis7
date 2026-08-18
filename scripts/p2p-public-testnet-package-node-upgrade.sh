#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --node-root <path> \
    --package-deb <oasis7-linux-x64.deb> \
    --ops-tools-tar <oasis7-linux-x64-ops-tools.tar.gz> \
    --package-version <version> \
    --commit <sha> \
    --run-id <github-actions-run-id> \
    [--artifact-ref <ref>] \
    [--systemd-service <name>] \
    [--release-retention-count <count>] \
    [--restart-service] \
    [--post-restart-health-url <url>] \
    [--post-restart-status-url <url>] \
    [--post-restart-timeout-secs <secs>]

Description:
  Upgrade an installed public testnet Linux node from a CI package bundle.
  The script extracts the Debian player package and checksummed operator tools
  into <node-root>/releases/<package-version>,
  repoints <node-root>/current, and rewrites the node-local governed bootstrap
  bundle runtime_build hash to the installed runtime binary. This keeps the
  network-tier runtime drift guard aligned with the deployed artifact.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

safe_ops_tools_extract() {
  local archive=$1 destination=$2
  python3 "$SCRIPT_DIR/p2p-safe-extract-tar.py" "$archive" "$destination" \
    || die "cannot safely extract ops-tools archive"
}

safe_deb_extract() {
  local package=$1 destination=$2
  command -v dpkg-deb >/dev/null 2>&1 || die "dpkg-deb is required to extract the Debian package"
  dpkg-deb --extract "$package" "$destination" \
    || die "cannot extract Debian package"
  # No package member is hashed, copied, or executed until this complete
  # physical-tree pass has rejected symlinks, special files, and path escapes.
  python3 "$SCRIPT_DIR/p2p-safe-validate-deb-tree.py" "$destination" "opt/oasis7" \
    || die "extracted Debian package failed the symlink/non-regular/path containment checks"
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

journal_transaction_phase() {
  local transaction_dir=$1
  local phase=$2
  local promoted_current=${3:-}
  python3 - "$transaction_dir/transaction.json" "$phase" "$promoted_current" <<'PY'
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
phase = sys.argv[2]
promoted_current = sys.argv[3] if len(sys.argv) > 3 else ""
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
manifest["phase"] = phase
if promoted_current:
    manifest["promoted_current"] = {
        "kind": "symlink",
        # Preserve the exact target spelling used for the in-flight symlink.
        # The resolved field is the canonical safety identity; target spelling
        # remains observable evidence and may contain a caller-provided double
        # slash on macOS temporary paths.
        "target": promoted_current,
        "resolved": str(Path(promoted_current).resolve(strict=False)),
    }
temporary = manifest_path.with_name(f".{manifest_path.name}.{os.getpid()}.tmp")
payload = (json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8")
with temporary.open("wb") as handle:
    handle.write(payload)
    handle.flush()
    os.fsync(handle.fileno())
os.replace(temporary, manifest_path)
directory_fd = os.open(manifest_path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
try:
    os.fsync(directory_fd)
finally:
    os.close(directory_fd)
PY
}

create_transaction_snapshot() {
  local root=$1
  local transaction_dir=$2
  python3 - "$root" "$transaction_dir" <<'PY'
from __future__ import annotations

import json
import os
import shutil
import stat
import sys
from pathlib import Path

root = Path(sys.argv[1])
transaction_dir = Path(sys.argv[2])
snapshot_dir = transaction_dir / "snapshot"
snapshot_dir.mkdir(parents=True, exist_ok=False)

bundle_paths = sorted(
    (root / "config").rglob("public-testnet-governed-bootstrap-bundle-2026-06-06.json")
)
if not bundle_paths:
    raise SystemExit(f"no governed bootstrap bundle found under {root / 'config'}")

def snapshot_file(path: Path) -> dict[str, object]:
    relative = path.relative_to(root)
    backup = snapshot_dir / "files" / relative
    entry: dict[str, object] = {
        "path": str(relative),
        "backup": str(backup.relative_to(transaction_dir)),
        "present": path.is_file(),
    }
    if path.is_file():
        metadata = path.stat()
        backup.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, backup)
        with backup.open("rb") as handle:
            os.fsync(handle.fileno())
        parent_fd = os.open(backup.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
        entry["uid"] = metadata.st_uid
        entry["gid"] = metadata.st_gid
        entry["mode"] = stat.S_IMODE(metadata.st_mode)
    return entry

current = root / "current"
if current.is_symlink():
    current_state: dict[str, object] = {
        "kind": "symlink",
        "target": os.readlink(current),
        "resolved": str(current.resolve(strict=False)),
    }
elif current.is_dir():
    current_backup = snapshot_dir / "current-directory"
    shutil.copytree(current, current_backup, symlinks=True, copy_function=shutil.copy2)
    current_metadata = current.stat()
    shutil.copystat(current, current_backup, follow_symlinks=False)
    current_state = {
        "kind": "directory",
        "backup": str(current_backup.relative_to(transaction_dir)),
        "uid": current_metadata.st_uid,
        "gid": current_metadata.st_gid,
        "mode": stat.S_IMODE(current_metadata.st_mode),
    }
elif current.is_file():
    current_backup = snapshot_dir / "current-file"
    shutil.copy2(current, current_backup)
    current_metadata = current.stat()
    current_state = {
        "kind": "file",
        "backup": str(current_backup.relative_to(transaction_dir)),
        "sha256": __import__("hashlib").sha256(current.read_bytes()).hexdigest(),
        "uid": current_metadata.st_uid,
        "gid": current_metadata.st_gid,
        "mode": stat.S_IMODE(current_metadata.st_mode),
    }
else:
    current_state = {"kind": "absent"}

manifest = {
    "schema_version": "oasis7.package_upgrade_rollback.v1",
    "phase": "snapshotted",
    "node_root": str(root),
    "current": current_state,
    "files": [
        *(snapshot_file(path) for path in bundle_paths),
        snapshot_file(root / "CURRENT_VERSION"),
        snapshot_file(root / "DEPLOYED_BUILDINFO"),
    ],
}
manifest_path = transaction_dir / "transaction.json"
payload = (json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8")
with manifest_path.open("wb") as handle:
    handle.write(payload)
    handle.flush()
    os.fsync(handle.fileno())
directory_fd = os.open(transaction_dir, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
try:
    os.fsync(directory_fd)
finally:
    os.close(directory_fd)
PY
}

rollback_transaction() {
  local transaction_dir=$1
  python3 - "$transaction_dir" <<'PY'
from __future__ import annotations

import json
import os
import shutil
import stat
import sys
from pathlib import Path

transaction_dir = Path(sys.argv[1])
manifest_path = transaction_dir / "transaction.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if not isinstance(manifest, dict) or manifest.get("schema_version") != "oasis7.package_upgrade_rollback.v1":
    raise RuntimeError(
        "rollback manifest has unsupported schema_version; refusing mutation"
    )
root = Path(manifest["node_root"])

def write_phase(phase: str) -> None:
    manifest["phase"] = phase
    temporary = manifest_path.with_name(f".{manifest_path.name}.{os.getpid()}.tmp")
    payload = (json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8")
    with temporary.open("wb") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, manifest_path)
    directory_fd = os.open(manifest_path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)

def current_identity(path: Path) -> dict[str, object]:
    if path.is_symlink():
        return {"kind": "symlink", "target": os.readlink(path), "resolved": str(path.resolve(strict=False))}
    if path.is_dir():
        metadata = path.stat()
        return {"kind": "directory", "resolved": str(path.resolve(strict=False)), "uid": metadata.st_uid, "gid": metadata.st_gid, "mode": stat.S_IMODE(metadata.st_mode)}
    if path.is_file():
        metadata = path.stat()
        return {"kind": "file", "resolved": str(path.resolve(strict=False)), "uid": metadata.st_uid, "gid": metadata.st_gid, "mode": stat.S_IMODE(metadata.st_mode), "sha256": __import__("hashlib").sha256(path.read_bytes()).hexdigest()}
    return {"kind": "absent"}

def current_matches(
    path: Path,
    expected: dict[str, object],
    *,
    allow_symlink_spelling: bool = False,
) -> bool:
    actual = current_identity(path)
    kind = expected.get("kind")
    if actual.get("kind") != kind:
        return False
    fields = {
        "symlink": ("resolved",) if allow_symlink_spelling else ("target", "resolved"),
        "directory": ("resolved", "uid", "gid", "mode"),
        "file": ("resolved", "uid", "gid", "mode", "sha256"),
        "absent": (),
    }[str(kind)]
    return all(actual.get(field) == expected.get(field) for field in fields)

def current_matches_resolved(path: Path, expected: dict[str, object]) -> bool:
    actual = current_identity(path)
    return actual.get("kind") == expected.get("kind") and actual.get("resolved") == expected.get("resolved")

current = root / "current"
current_state = manifest.get("current")
if not isinstance(current_state, dict):
    raise RuntimeError("rollback manifest has invalid current state")
promoted_current = manifest.get("promoted_current")
expected_current = promoted_current if isinstance(promoted_current, dict) else current_state
current_phase = str(manifest.get("phase", ""))
allow_promoted_spelling = current_phase in {
    "current_promotion_intent",
    "current_promoted",
    "current_normalized",
}
current_matches_expected = current_matches(
    current,
    expected_current,
    allow_symlink_spelling=allow_promoted_spelling,
)
current_matches_snapshot = current_matches(current, current_state)
if current_phase == "current_promotion_intent":
    # The write-ahead intent is durable before removing the old link.  A crash
    # can therefore leave either the exact snapshot current or the promoted
    # target exposed; both are safe rollback-recognizable states.  Any other
    # identity remains external drift and fails closed.
    if not (current_matches_expected or current_matches_snapshot or current_matches_resolved(current, expected_current)):
        raise RuntimeError("current identity drift detected; refusing rollback")
elif not (current_matches_expected or current_matches_resolved(current, expected_current)):
    raise RuntimeError("current identity drift detected; refusing rollback")

def remove_path(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists():
        shutil.rmtree(path)

def restore_file(entry: dict[str, object]) -> None:
    destination = root / str(entry["path"])
    if not entry["present"]:
        if destination.is_symlink() or destination.is_file():
            destination.unlink()
        elif destination.exists():
            raise RuntimeError(f"refusing to remove non-file rollback target: {destination}")
        return
    source = transaction_dir / str(entry["backup"])
    if not source.is_file():
        raise RuntimeError(f"rollback snapshot missing: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(f".{destination.name}.rollback-{os.getpid()}.tmp")
    shutil.copy2(source, temporary)
    os.chown(temporary, int(entry["uid"]), int(entry["gid"]))
    os.chmod(temporary, int(entry["mode"]))
    os.replace(temporary, destination)

write_phase("rollback_started")
for entry in manifest["files"]:
    restore_file(entry)

current = root / "current"
remove_path(current)
current_state = manifest["current"]
kind = current_state["kind"]
if kind == "symlink":
    os.symlink(str(current_state["target"]), current)
elif kind == "directory":
    source = transaction_dir / str(current_state["backup"])
    temporary = current.with_name(f".{current.name}.rollback-{os.getpid()}.tmp")
    shutil.copytree(source, temporary, symlinks=True, copy_function=shutil.copy2)
    os.chown(temporary, int(current_state["uid"]), int(current_state["gid"]))
    os.chmod(temporary, int(current_state["mode"]))
    os.replace(temporary, current)
elif kind == "file":
    source = transaction_dir / str(current_state["backup"])
    temporary = current.with_name(f".{current.name}.rollback-{os.getpid()}.tmp")
    shutil.copy2(source, temporary)
    os.chown(temporary, int(current_state["uid"]), int(current_state["gid"]))
    os.chmod(temporary, int(current_state["mode"]))
    os.replace(temporary, current)
elif kind != "absent":
    raise RuntimeError(f"unknown current rollback kind: {kind}")

write_phase("rolled_back")
PY
}

transaction_dir=""
transaction_active=0
transaction_completed=0

handle_upgrade_exit() {
  local status=$?
  trap - EXIT
  if [[ "$status" -ne 0 && "$transaction_active" -eq 1 && "$transaction_completed" -eq 0 ]]; then
    echo "package_upgrade_rollback_begin=true transaction_dir=$transaction_dir" >&2
    set +e
    rollback_status=0
    if [[ "$restart_service" -eq 1 ]]; then
      systemctl stop "$systemd_service"
      rollback_status=$?
      if [[ "$rollback_status" -eq 0 ]]; then
        assert_no_node_processes "$node_root"
        rollback_status=$?
      fi
    fi
    if [[ "$rollback_status" -eq 0 ]]; then
      rollback_transaction "$transaction_dir"
      rollback_status=$?
    fi
    if [[ "$rollback_status" -eq 0 && "$restart_service" -eq 1 ]]; then
      systemctl daemon-reload
      rollback_status=$?
      if [[ "$rollback_status" -eq 0 ]]; then
        systemctl start "$systemd_service"
        rollback_status=$?
      fi
    fi
    set -e
    if [[ "$rollback_status" -eq 0 ]]; then
      echo "package_upgrade_rollback_complete=true transaction_dir=$transaction_dir" >&2
    else
      echo "package_upgrade_rollback_failed=true transaction_dir=$transaction_dir status=$rollback_status" >&2
      status=$rollback_status
    fi
  fi
  cleanup_upgrade_lock
  exit "$status"
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

normalize_promoted_current_link() {
  local current_path=$1
  local promoted_target=$2
  local canonical_target=$3
  python3 - "$current_path" "$promoted_target" "$canonical_target" <<'PY'
from __future__ import annotations

import os
import sys
from pathlib import Path

current = Path(sys.argv[1])
promoted_target = sys.argv[2]
canonical_target = Path(sys.argv[3])
if not current.is_symlink():
    raise SystemExit(f"current is not the promoted symlink: {current}")
if os.readlink(current) != promoted_target:
    raise SystemExit("current target changed before commit normalization")
if current.resolve(strict=False) != canonical_target.resolve(strict=False):
    raise SystemExit("current resolved target changed before commit normalization")

temporary = current.with_name(f".{current.name}.normalize-{os.getpid()}.tmp")
try:
    os.symlink(str(canonical_target), temporary)
    os.replace(temporary, current)
    directory_fd = os.open(current.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)
finally:
    if temporary.is_symlink() or temporary.exists():
        temporary.unlink()
PY
}

promote_current_link() {
  local current_path=$1
  local promoted_target=$2
  local directory_backup=$3
  python3 - "$current_path" "$promoted_target" "$directory_backup" <<'PY'
from __future__ import annotations

import os
import sys
from pathlib import Path

current = Path(sys.argv[1])
promoted_target = sys.argv[2]
directory_backup = Path(sys.argv[3])
temporary = current.with_name(f".{current.name}.promote-{os.getpid()}.tmp")

def fsync_parent(path: Path) -> None:
    directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)

def present(path: Path) -> bool:
    return path.is_symlink() or path.exists()

if present(temporary):
    raise RuntimeError(f"promotion temporary already exists: {temporary}")

try:
    os.symlink(promoted_target, temporary)
    fsync_parent(temporary)

    # A non-symlink directory cannot be atomically replaced by a symlink while
    # retaining its governed backup.  Rename it to the durable backup first,
    # then atomically expose the prepared symlink.  Any failure after the
    # backup rename is intentionally fail-closed: rollback must not guess
    # whether the replacement reached disk; an operator must recover the
    # explicitly named backup/current paths.
    if current.is_dir() and not current.is_symlink():
        if present(directory_backup):
            raise RuntimeError(f"directory backup already exists: {directory_backup}")
        os.replace(current, directory_backup)
        fsync_parent(current)
        try:
            os.replace(temporary, current)
        except Exception as exc:
            raise RuntimeError(
                "directory current promotion incomplete; manual recovery required"
            ) from exc
        fsync_parent(current)
    else:
        # Symlink, regular file, absent current, and dangling symlink all use
        # same-directory temporary symlink + atomic replacement, so current is
        # never intentionally absent during this promotion path.
        os.replace(temporary, current)
        fsync_parent(current)
finally:
    if temporary.is_symlink() or temporary.exists():
        temporary.unlink()
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
package_deb=""
ops_tools_tar=""
package_version=""
commit=""
run_id=""
artifact_ref=""
systemd_service=""
release_retention_count=3
restart_service=0
post_restart_status_url=""
post_restart_health_url=""
post_restart_timeout_secs=60

while [[ $# -gt 0 ]]; do
  case "$1" in
    --node-root)
      node_root=${2:-}
      shift 2
      ;;
    --package-deb)
      package_deb=${2:-}
      shift 2
      ;;
    --ops-tools-tar)
      ops_tools_tar=${2:-}
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
    --post-restart-health-url)
      post_restart_health_url=${2:-}
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
require_non_empty "--package-deb" "$package_deb"
require_non_empty "--ops-tools-tar" "$ops_tools_tar"
require_non_empty "--package-version" "$package_version"
require_non_empty "--commit" "$commit"
require_non_empty "--run-id" "$run_id"
[[ -f "$package_deb" ]] || die "missing Debian package: $package_deb"
[[ -f "$ops_tools_tar" ]] || die "missing ops-tools package: $ops_tools_tar"
if [[ "$restart_service" -eq 1 ]]; then
  require_non_empty "--systemd-service" "$systemd_service"
fi
if [[ -n "$post_restart_status_url" && "$restart_service" -ne 1 ]]; then
  die "--post-restart-status-url requires --restart-service"
fi
if [[ -n "$post_restart_health_url" && "$restart_service" -ne 1 ]]; then
  die "--post-restart-health-url requires --restart-service"
fi
if [[ -n "$post_restart_status_url" && -n "$post_restart_health_url" ]]; then
  die "--post-restart-status-url and --post-restart-health-url are mutually exclusive"
fi
if [[ -n "$post_restart_health_url" && "$post_restart_health_url" != */healthz ]]; then
  die "--post-restart-health-url must target an explicit /healthz endpoint"
fi
if [[ ! "$post_restart_timeout_secs" =~ ^[0-9]+$ || "$post_restart_timeout_secs" -le 0 ]]; then
  die "--post-restart-timeout-secs must be a positive integer"
fi
if [[ ! "$release_retention_count" =~ ^[0-9]+$ ]]; then
  die "--release-retention-count must be a non-negative integer"
fi

node_root_lexical=$(python3 -c 'import os,sys; value=os.path.expanduser(sys.argv[1]); print(value if os.path.isabs(value) else os.path.join(os.getcwd(), value))' "$node_root")
node_root=$(abs_path "$node_root")
package_deb=$(abs_path "$package_deb")
ops_tools_tar=$(abs_path "$ops_tools_tar")
artifact_ref=${artifact_ref:-"oasis7-linux-x64.deb!/opt/oasis7/bin/oasis7_chain_runtime"}
node_id=$(parse_node_id "$node_root/bin/start-node.sh")
upgrade_lock_dir="$node_root/.package-upgrade.lock"
if ! mkdir "$upgrade_lock_dir" 2>/dev/null; then
  die "another package upgrade is already running for $node_root"
fi
trap handle_upgrade_exit EXIT

release_dir="$node_root/releases/$package_version"
tmp_dir="$node_root/releases/.${package_version}.tmp.$$"
backup_suffix="pre-${package_version//[^A-Za-z0-9_.-]/_}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
transaction_dir="$node_root/package-upgrade-rollback/$backup_suffix"

mkdir -p "$node_root/releases"
rm -rf "$tmp_dir"
mkdir -p "$tmp_dir"
command -v dpkg-deb >/dev/null 2>&1 || die "dpkg-deb is required to extract the Debian package"
safe_deb_extract "$package_deb" "$tmp_dir/deb-root"
bundle_root="$tmp_dir/deb-root/opt/oasis7"
[[ -d "$bundle_root" ]] || die "Debian package missing /opt/oasis7 player bundle: $package_deb"
# BUILDINFO and SHA256SUMS are embedded in the Debian payload.  Bind all three
# CLI provenance fields before any ops-tool copy, transaction snapshot, or
# deployment metadata write can occur.
python3 "$SCRIPT_DIR/p2p-verify-linux-package-bundle.py" \
  "$bundle_root" "$package_version" "$commit" "$run_id" \
  || die "embedded Debian BUILDINFO/SHA256SUMS provenance verification failed"
safe_ops_tools_extract "$ops_tools_tar" "$tmp_dir"
ops_bundle_root="$tmp_dir/oasis7-linux-x64-ops-tools"
[[ -f "$ops_bundle_root/.oasis7-ops-tools-manifest.json" ]] || die "ops-tools archive missing manifest"
[[ -f "$ops_bundle_root/SHA256SUMS" ]] || die "ops-tools archive missing SHA256SUMS"
(
  cd "$ops_bundle_root"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c SHA256SUMS >/dev/null 2>&1 || die "ops-tools checksum verification failed"
  else
    shasum -a 256 -c SHA256SUMS >/dev/null 2>&1 || die "ops-tools checksum verification failed"
  fi
)
for ops_binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  [[ -x "$ops_bundle_root/bin/$ops_binary" ]] || die "ops-tools archive missing executable: $ops_binary"
done
mkdir -p "$bundle_root/bin"
cp -a "$ops_bundle_root/bin/." "$bundle_root/bin/"
runtime_bin="$bundle_root/bin/oasis7_chain_runtime"
[[ -x "$runtime_bin" ]] || die "bundle missing executable runtime: $runtime_bin"
ensure_governed_bootstrap_bundle_exists "$node_root"

# The durable rollback snapshot is the boundary for all service-manager calls.
# Keep setup/preflight failures side-effect free with respect to systemd.  This
# also means a stop/restart failure can use handle_upgrade_exit's existing
# transaction rollback path without guessing the previous current release.
mkdir -p "$(dirname "$transaction_dir")"
create_transaction_snapshot "$node_root" "$transaction_dir"
transaction_active=1

if [[ "$restart_service" -eq 1 ]]; then
  systemctl daemon-reload
  systemctl stop "$systemd_service"
  sleep 2
  assert_no_node_processes "$node_root"
fi

python3 - "$node_root" "$bundle_root" "$package_version" "$commit" "$run_id" "$artifact_ref" "$transaction_dir" <<'PY'
from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

node_root = Path(sys.argv[1])
bundle_root = Path(sys.argv[2])
package_version = sys.argv[3]
commit = sys.argv[4]
run_id = sys.argv[5]
artifact_ref = sys.argv[6]
transaction_dir = Path(sys.argv[7])
runtime_bin = bundle_root / "bin" / "oasis7_chain_runtime"

transaction_manifest = json.loads(
    (transaction_dir / "transaction.json").read_text(encoding="utf-8")
)
metadata_by_path = {
    str(entry["path"]): entry
    for entry in transaction_manifest["files"]
    if entry.get("present")
}

def atomic_write_preserving_metadata(path: Path, content: str) -> None:
    relative = str(path.relative_to(node_root))
    entry = metadata_by_path.get(relative)
    temporary = path.with_name(f".{path.name}.promote-{os.getpid()}.tmp")
    payload = content.encode("utf-8")
    try:
        with temporary.open("wb") as handle:
            written = 0
            while written < len(payload):
                written += handle.write(payload[written:])
            if entry is not None:
                os.fchown(handle.fileno(), int(entry["uid"]), int(entry["gid"]))
                os.fchmod(handle.fileno(), int(entry["mode"]))
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        if temporary.exists():
            temporary.unlink()

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
    runtime["package_version"] = package_version
    runtime["run_id"] = run_id
    runtime["updated_by"] = updated_by
    data["git_commit"] = commit
    data["updated_by"] = updated_by
    atomic_write_preserving_metadata(
        bundle_path,
        json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
    )

atomic_write_preserving_metadata(node_root / "CURRENT_VERSION", package_version + "\n")
atomic_write_preserving_metadata(
    node_root / "DEPLOYED_BUILDINFO",
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
)
print(f"runtime_sha256={runtime_sha}")
print(f"runtime_size={runtime_size}")
print(f"updated_bundle_count={len(bundle_paths)}")
PY
journal_transaction_phase "$transaction_dir" "metadata_promoted"

rm -rf "$release_dir"
mv "$bundle_root" "$release_dir"
rm -rf "$tmp_dir"
journal_transaction_phase "$transaction_dir" "release_promoted"

current_path="$node_root/current"
previous_current_path=""
promoted_current_path="$node_root/releases/$package_version"
promoted_current_target="$node_root_lexical/releases/$package_version"
journal_transaction_phase "$transaction_dir" "current_promotion_intent" "$promoted_current_target"
if [[ -L "$current_path" || -e "$current_path" ]]; then
  previous_current_path="$(readlink -f "$current_path" || true)"
  if [[ -n "$previous_current_path" ]]; then
    printf '%s\n' "$previous_current_path" >"$node_root/last-$backup_suffix.txt"
  fi
fi
promote_current_link "$current_path" "$promoted_current_target" "$node_root/current-$backup_suffix.dir"
journal_transaction_phase "$transaction_dir" "current_promoted" "$promoted_current_target"

migrate_legacy_replication_root "$node_root" "$node_id"
journal_transaction_phase "$transaction_dir" "replication_migrated"
prune_old_releases "$node_root/releases" "$current_path" "$release_retention_count" "$previous_current_path"
journal_transaction_phase "$transaction_dir" "releases_pruned"

if [[ "$restart_service" -eq 1 ]]; then
  systemctl daemon-reload
  systemctl start "$systemd_service"
  sleep 3
  systemctl is-active --quiet "$systemd_service"
  systemctl --no-pager --full status "$systemd_service" | sed -n '1,18p'
  if [[ -n "$post_restart_health_url" ]]; then
    deadline=$((SECONDS + post_restart_timeout_secs))
    last_health=""
    while (( SECONDS < deadline )); do
      if health_json="$(curl -fsS --max-time 5 "$post_restart_health_url" 2>/dev/null)"; then
        last_health="$health_json"
        if jq -e '.ok == true' >/dev/null <<<"$health_json"; then
          echo "post_restart_health=ok"
          break
        fi
      fi
      sleep 3
    done
    if [[ -z "$last_health" ]] || ! jq -e '.ok == true' >/dev/null <<<"$last_health"; then
      echo "error: post-restart health did not become ok before timeout" >&2
      if [[ -n "$last_health" ]]; then
        jq -S . <<<"$last_health" >&2 || true
      fi
      exit 1
    fi
  elif [[ -n "$post_restart_status_url" ]]; then
    deadline=$((SECONDS + post_restart_timeout_secs))
    last_status=""
    while (( SECONDS < deadline )); do
      if status_json="$(curl -fsS --max-time 5 "$post_restart_status_url" 2>/dev/null)"; then
        last_status="$status_json"
        if jq -e '
          .running == true
          and (.last_error == null or .last_error == "null")
          and (.readiness.status // null) == "ready"
          and ((.consensus.committed_height // 0) > 0)
          and ((.consensus.last_execution_height // 0) > 0)
          and ((.consensus.last_execution_block_hash // "") != "")
          and ((.consensus.last_execution_state_root // "") != "")
          and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
          and ((.world_resource.readiness_status // null) == "ready")
          and (((.world_resource.failed_gates // []) | length) == 0)
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
      and ((.consensus.committed_height // 0) > 0)
      and ((.consensus.last_execution_height // 0) > 0)
      and ((.consensus.last_execution_block_hash // "") != "")
      and ((.consensus.last_execution_state_root // "") != "")
      and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
      and ((.world_resource.readiness_status // null) == "ready")
      and (((.world_resource.failed_gates // []) | length) == 0)
      and (.consensus.storage_challenge_network_degraded_height // null) == null
      and ((.observability.storage_challenge_network_degraded // false) | not)
    ' >/dev/null <<<"$last_status"; then
      echo "error: post-restart status did not become ready before timeout" >&2
      if [[ -n "$last_status" ]]; then
        jq -S '{running,last_error,readiness,consensus:{committed_height:.consensus.committed_height,network_committed_height:.consensus.network_committed_height,last_block_hash:.consensus.last_block_hash,last_execution_height:.consensus.last_execution_height,last_execution_block_hash:.consensus.last_execution_block_hash,last_execution_state_root:.consensus.last_execution_state_root,network_head:.consensus.network_head,storage_challenge_network_degraded_height:.consensus.storage_challenge_network_degraded_height,storage_challenge_network_degraded_reason:.consensus.storage_challenge_network_degraded_reason},world_resource:{readiness_status:.world_resource.readiness_status,failed_gates:.world_resource.failed_gates,last_delta_commit_height:.world_resource.last_delta_commit_height},observability:{storage_challenge_network_degraded:.observability.storage_challenge_network_degraded}}' <<<"$last_status" >&2 || true
      fi
      exit 1
    fi
  fi
fi

# Keep the lexical target spelling during the in-flight transaction so service
# stop/quiescence evidence identifies exactly the promoted path.  Canonicalize
# only after all post-start readiness checks pass, before committing the
# transaction; this preserves existing canonical readback semantics while
# making rollback evidence unambiguous across symlinked platform prefixes.
normalize_promoted_current_link "$current_path" "$promoted_current_target" "$promoted_current_path"
journal_transaction_phase "$transaction_dir" "current_normalized" "$promoted_current_path"
journal_transaction_phase "$transaction_dir" "completed"
transaction_completed=1
echo "upgraded $node_root to $package_version"
