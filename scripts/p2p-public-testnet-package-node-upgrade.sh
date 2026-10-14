#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR

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

  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --operation-mode stage \
    --operation-id <id> --action-id <id> \
    --expected-instance-id <id> --expected-generation <number> \
    [--operation-request-digest <sha256>] \
    --node-root <path> --package-deb <path> --ops-tools-tar <path> \
    --package-version <version> --commit <sha> --run-id <id>

  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --operation-mode stage --operation-profile local_native_package_v1 \
    --operation-id <id> --action-id <id> \
    --node-root <path> --native-runtime <executable> \
    --native-runtime-sha256 <sha256> --native-buildinfo <file> \
    --native-buildinfo-sha256 <sha256> --package-version <version> \
    --commit <sha> --run-id <id> \
    [--candidate-chain-plan <json> --candidate-chain-plan-sha256 <sha256>]

  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --operation-mode promote --operation-profile <profile> \
    --operation-id <id> --action-id <stage-action-id> \
    --operation-record-digest <sha256> --operation-request-digest <sha256> \
    --lifecycle-data-dir <path> --expected-incarnation <id> \
    --expected-generation <number> --expected-config-digest <sha256> \
    --expected-instance-action-id <id> --promotion-action-id <id> \
    --stop-action-id <id> --node-root <path>

  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --operation-mode rollback --operation-profile <profile> \
    --operation-id <id> --action-id <stage-action-id> \
    --operation-record-digest <sha256> --operation-request-digest <sha256> \
    --lifecycle-data-dir <path> --expected-incarnation <id> \
    --expected-generation <number> --expected-config-digest <sha256> \
    --expected-instance-action-id <id> --promotion-action-id <id> \
    --stop-action-id <id> --failed-start-action-id <id> \
    --failed-start-receipt-sha256 <sha256> --rollback-action-id <id> --node-root <path>

  ./scripts/p2p-public-testnet-package-node-upgrade.sh \
    --operation-mode query --operation-id <id> --action-id <id> \
    --operation-request-digest <sha256> --node-root <path>

Description:
  Stage a public testnet Linux package or a separately selected local-native
  runtime into a private, immutable operation transaction. The Linux profile
  extracts the Debian player package and checksummed operator tools; the
  local-native profile binds one executable and BUILDINFO to their hashes.
  Neither staging path stops or starts a service.

Promotion requires the exact immutable operation request plus the original
target host's stopped-instance receipt. It writes a durable recovery intent,
snapshots the preimage, promotes the immutable release, and records a receipt.
The local node operation adapter starts the promoted executable and verifies
the actual running supervisor and runtime process images against that receipt.
If that exact promoted start terminates, an operation-bound rollback restores
the snapshotted current/config preimage and returns the prior runtime identity.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

sha256_file() {
  local path=$1
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  else
    shasum -a 256 "$path" | awk '{print $1}'
  fi
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

require_safe_package_version() {
  local value=$1
  # Keep the release name a single portable path component.  This validation
  # must precede every release/tmp/rollback path interpolation below; the
  # embedded BUILDINFO verifier applies the same rule to package-supplied
  # provenance before bootstrap callers use that value as a release name.
  if [[ ! "$value" =~ ^[A-Za-z0-9][A-Za-z0-9_.+:~-]*$ || "$value" == *..* ]]; then
    die "--package-version must be a safe single path token (no separators or dot traversal): $value"
  fi
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
import subprocess
import sys
import tempfile
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
if current_phase in {"current_promotion_intent", "rollback_started", "rolled_back"}:
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

operation_transaction_tool() {
  python3 - "$@" <<'PY'
from __future__ import annotations

import hashlib
import fcntl
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(message)


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("utf-8")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    if path.is_symlink() or not path.is_file():
        fail(f"operation artifact is not a regular file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def open_directory_nofollow(path: Path) -> int:
    """Open every absolute path component relative to retained directory FDs."""
    if not path.is_absolute() or ".." in path.parts:
        fail(f"package path must be absolute and normalized: {path}")
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    nofollow = getattr(os, "O_NOFOLLOW", 0)
    current_fd = os.open("/", flags | nofollow)
    try:
        for component in path.parts:
            if component in ("/", ""):
                continue
            next_fd = os.open(component, flags | nofollow, dir_fd=current_fd)
            os.close(current_fd)
            current_fd = next_fd
        return current_fd
    except Exception:
        os.close(current_fd)
        raise


def open_directory_at(parent_fd: int, name: str, description: str) -> int:
    if not name or name in (".", "..") or "/" in name:
        fail(f"invalid package {description} path component")
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    try:
        return os.open(name, flags | getattr(os, "O_NOFOLLOW", 0), dir_fd=parent_fd)
    except OSError as error:
        fail(f"package {description} directory is unavailable without following symlinks: {error}")


def open_regular_at(parent_fd: int, name: str, description: str) -> tuple[int, os.stat_result]:
    if not name or name in (".", "..") or "/" in name:
        fail(f"invalid package {description} path component")
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NONBLOCK", 0)
    try:
        descriptor = os.open(name, flags | getattr(os, "O_NOFOLLOW", 0), dir_fd=parent_fd)
    except OSError as error:
        fail(f"package {description} is unavailable without following symlinks: {error}")
    metadata = os.fstat(descriptor)
    if not stat.S_ISREG(metadata.st_mode):
        os.close(descriptor)
        fail(f"package {description} is not a regular file")
    return descriptor, metadata


def metadata_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
        metadata.st_uid,
    )


def digest_descriptor(descriptor: int) -> str:
    os.lseek(descriptor, 0, os.SEEK_SET)
    digest = hashlib.sha256()
    while True:
        chunk = os.read(descriptor, 1024 * 1024)
        if not chunk:
            break
        digest.update(chunk)
    return digest.hexdigest()


def read_descriptor(descriptor: int, maximum: int, description: str) -> bytes:
    os.lseek(descriptor, 0, os.SEEK_SET)
    chunks: list[bytes] = []
    remaining = maximum + 1
    while remaining:
        chunk = os.read(descriptor, min(remaining, 65536))
        if not chunk:
            break
        chunks.append(chunk)
        remaining -= len(chunk)
    value = b"".join(chunks)
    if len(value) > maximum:
        fail(f"package {description} exceeds its byte bound")
    return value


def named_component_matches_fd(parent_fd: int, name: str, descriptor: int, description: str) -> None:
    if not name or name in (".", "..") or "/" in name:
        fail(f"invalid package {description} path component")
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"package {description} pathname changed after it was opened: {error}")
    if (
        named.st_dev != opened.st_dev
        or named.st_ino != opened.st_ino
        or stat.S_IFMT(named.st_mode) != stat.S_IFMT(opened.st_mode)
    ):
        fail(f"package {description} pathname no longer names the opened file")


def path_still_names_directory(path: Path, descriptor: int, description: str) -> None:
    try:
        reopened = open_directory_nofollow(path)
        try:
            actual = os.fstat(descriptor)
            named = os.fstat(reopened)
        finally:
            os.close(reopened)
    except OSError as error:
        fail(f"package {description} path changed during the snapshot: {error}")
    if actual.st_dev != named.st_dev or actual.st_ino != named.st_ino:
        fail(f"package {description} path no longer names the opened directory")


def parse_buildinfo_bytes(value: bytes) -> dict[str, str]:
    try:
        lines = value.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        fail(f"prior package BUILDINFO is not UTF-8: {error}")
    fields: dict[str, str] = {}
    for line_number, line in enumerate(lines, 1):
        if "=" not in line:
            fail(f"prior package BUILDINFO line {line_number} is malformed")
        name, field_value = line.split("=", 1)
        if not name.strip() or name in fields:
            fail("prior package BUILDINFO has an empty or duplicate field")
        fields[name] = field_value
    return fields


def release_name_from_link(root: Path, target: str) -> str:
    target_path = Path(target)
    if not target_path.is_absolute():
        target_path = root / target_path
    releases_path = root / "releases"
    if target_path.parent != releases_path:
        fail("package current must directly select one child of releases")
    name = target_path.name
    if name in ("", ".", ".."):
        fail("package current has an invalid prior release name")
    return name


def read_release_snapshot_at(
    root: Path,
    root_fd: int,
    release_name: str,
    profile: str | None,
    *,
    require_provenance: bool,
) -> dict[str, object]:
    releases_fd = open_directory_at(root_fd, "releases", "releases")
    release_fd = bin_fd = runtime_fd = buildinfo_fd = -1
    try:
        release_fd = open_directory_at(releases_fd, release_name, "release")
        bin_fd = open_directory_at(release_fd, "bin", "runtime bin")
        runtime_fd, runtime_before = open_regular_at(bin_fd, "oasis7_chain_runtime", "runtime")
        if runtime_before.st_mode & 0o111 == 0:
            fail("prior package runtime is not executable")
        runtime_sha256 = digest_descriptor(runtime_fd)
        runtime_after = os.fstat(runtime_fd)
        if metadata_identity(runtime_before) != metadata_identity(runtime_after):
            fail("prior package runtime changed while hashing its open descriptor")
        release_path = root / "releases" / release_name
        runtime_path = release_path / "bin" / "oasis7_chain_runtime"
        result: dict[str, object] = {
            "runtime_path": str(runtime_path),
            "runtime_sha256": runtime_sha256,
            "runtime_size": runtime_before.st_size,
        }
        if require_provenance:
            buildinfo_fd, buildinfo_before = open_regular_at(release_fd, "BUILDINFO", "BUILDINFO")
            if buildinfo_before.st_size > 1024 * 1024:
                fail("prior package BUILDINFO exceeds its byte bound")
            buildinfo_bytes = read_descriptor(buildinfo_fd, 1024 * 1024, "BUILDINFO")
            buildinfo_after = os.fstat(buildinfo_fd)
            if metadata_identity(buildinfo_before) != metadata_identity(buildinfo_after):
                fail("prior package BUILDINFO changed while reading its open descriptor")
            fields = parse_buildinfo_bytes(buildinfo_bytes)
            expected_platform = "local-native" if profile == "local_native_package_v1" else "linux-x64"
            if (
                fields.get("platform") != expected_platform
                or fields.get("runtime_sha256") != runtime_sha256
                or not fields.get("package_version")
                or not fields.get("commit")
                or not fields.get("run_id")
            ):
                fail("prior package runtime does not match its BUILDINFO provenance")
            result.update({
                "package_version": fields["package_version"],
                "commit": fields["commit"],
                "platform": expected_platform,
                "buildinfo_sha256": digest_bytes(buildinfo_bytes),
            })
        named_component_matches_fd(root_fd, "releases", releases_fd, "releases")
        named_component_matches_fd(releases_fd, release_name, release_fd, "prior release")
        named_component_matches_fd(release_fd, "bin", bin_fd, "runtime bin")
        named_component_matches_fd(bin_fd, "oasis7_chain_runtime", runtime_fd, "runtime")
        if buildinfo_fd >= 0:
            named_component_matches_fd(release_fd, "BUILDINFO", buildinfo_fd, "BUILDINFO")
        path_still_names_directory(root, root_fd, "node root")
        return result
    finally:
        for descriptor in (buildinfo_fd, runtime_fd, bin_fd, release_fd, releases_fd):
            if descriptor >= 0:
                os.close(descriptor)


def read_current_release_snapshot(root: Path, profile: str | None) -> dict[str, object]:
    root_fd = open_directory_nofollow(root)
    try:
        try:
            before = os.stat("current", dir_fd=root_fd, follow_symlinks=False)
        except OSError as error:
            fail(f"package current is unavailable: {error}")
        if not stat.S_ISLNK(before.st_mode):
            fail("package current must be a symlink to a valid prior release")
        try:
            target = os.readlink("current", dir_fd=root_fd)
        except OSError as error:
            fail(f"package current is not a readable publication symlink: {error}")
        release_name = release_name_from_link(root, target)
        release = read_release_snapshot_at(
            root, root_fd, release_name, profile, require_provenance=profile is not None
        )
        after = os.stat("current", dir_fd=root_fd, follow_symlinks=False)
        target_after = os.readlink("current", dir_fd=root_fd)
        if metadata_identity(before) != metadata_identity(after) or target_after != target:
            fail("package current changed while reading the selected release")
        path_still_names_directory(root, root_fd, "node root")
        resolved = str(root / "releases" / release_name)
        return {
            "current": {"kind": "symlink", "target": target, "resolved": resolved},
            "current_link_identity": {
                "device": before.st_dev,
                "inode": before.st_ino,
                "mode": before.st_mode,
                "size": before.st_size,
                "mtime_ns": before.st_mtime_ns,
                "ctime_ns": before.st_ctime_ns,
            },
            "current_runtime": {
                "present": True,
                "resolved": str(release["runtime_path"]),
                "sha256": str(release["runtime_sha256"]),
                "size": int(release["runtime_size"]),
            },
            "prior_runtime": {
                "restored_current_target": target,
                "restored_current_resolved": resolved,
                **{key: release[key] for key in ("package_version", "commit", "platform", "runtime_path", "runtime_sha256", "buildinfo_sha256") if key in release},
            } if profile is not None else None,
        }
    finally:
        os.close(root_fd)


def read_named_release_snapshot(
    root: Path,
    target: str,
    resolved: str,
    profile: str,
) -> dict[str, object]:
    release_name = release_name_from_link(root, target)
    release_path = root / "releases" / release_name
    if resolved != str(release_path):
        fail("prior package current target differs from its exact direct release path")
    root_fd = open_directory_nofollow(root)
    try:
        return read_release_snapshot_at(root, root_fd, release_name, profile, require_provenance=True)
    finally:
        os.close(root_fd)


def compatibility_probe_failure_detail(result: subprocess.CompletedProcess[bytes]) -> str:
    """Keep bounded diagnostics from both streams; the runtime logger may use stdout."""
    parts: list[str] = []
    for label, raw in (("stdout", result.stdout), ("stderr", result.stderr)):
        text = raw.decode("utf-8", errors="replace").strip()
        if text:
            parts.append(f"{label}: {text[:480]}")
    detail = " | ".join(parts)
    return detail[:1024] if detail else "(no stdout or stderr)"


def run_compatibility_descriptor(
    binary: Path,
    config_path: Path,
    data_dir: Path,
    expected_sha256: str,
    minimum_cut_bytes: bytes | None = None,
    minimum_cut_sha256: str | None = None,
) -> dict[str, object]:
    real_regular_file(config_path, "compatibility probe config")
    if config_path.resolve(strict=True) != config_path:
        fail("compatibility probe config path is not canonical")
    config_file_sha256 = digest_file(config_path)
    if (minimum_cut_bytes is None) != (minimum_cut_sha256 is None):
        fail("full-sync minimum-cut bytes and digest must be supplied together")
    minimum_cut_path: Path | None = None
    if minimum_cut_bytes is not None:
        if digest_bytes(minimum_cut_bytes) != minimum_cut_sha256:
            fail("full-sync minimum-cut bytes differ from the immutable staged digest")
        descriptor_fd, descriptor_path = tempfile.mkstemp(
            prefix="oasis7-stage-minimum-cut-", suffix=".json"
        )
        minimum_cut_path = Path(descriptor_path)
        try:
            os.fchmod(descriptor_fd, 0o600)
            with os.fdopen(descriptor_fd, "wb") as handle:
                handle.write(minimum_cut_bytes)
                handle.flush()
        except Exception:
            try:
                os.close(descriptor_fd)
            except OSError:
                pass
            minimum_cut_path.unlink(missing_ok=True)
            raise
    command = [
        str(binary),
        "node",
        "runtime-compatibility",
        "--config",
        str(config_path),
        "--data-dir",
        str(data_dir),
    ]
    if minimum_cut_path is not None:
        command.extend(
            [
                "--minimum-full-sync-cut",
                str(minimum_cut_path),
                "--minimum-full-sync-cut-sha256",
                str(minimum_cut_sha256),
            ]
        )
    try:
        result = subprocess.run(
            command,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
        )
    except Exception as exc:
        fail(f"cannot run executable compatibility probe for {binary}: {exc}")
    finally:
        if minimum_cut_path is not None:
            minimum_cut_path.unlink(missing_ok=True)
    if digest_file(config_path) != config_file_sha256:
        fail("node config changed while an executable compatibility probe was running")
    if result.returncode != 0:
        detail = compatibility_probe_failure_detail(result)
        fail(
            f"executable compatibility probe failed for {binary} "
            f"(exit status: {result.returncode}): {detail}"
        )
    try:
        value = json.loads(result.stdout)
    except Exception as exc:
        fail(f"executable compatibility probe returned invalid JSON: {exc}")
    if not isinstance(value, dict) or (
        value.get("api_version") != "oasis7.node-runtime-compatibility/v1"
        or value.get("executable_sha256") != expected_sha256
        or value.get("config_path") != str(config_path)
        or not isinstance(value.get("config_digest"), str)
        or len(value.get("config_digest", "")) != 64
        or value.get("config_sha256") != config_file_sha256
        or value.get("data_dir") != str(data_dir)
        or value.get("chain_authority_claim") is not False
        or value.get("process_started") is not False
        or value.get("runtime_profile") is None
        or not isinstance(value.get("software_formats"), dict)
        or not isinstance(value.get("stored_state_probe"), dict)
    ):
        fail(f"executable compatibility descriptor is incomplete or unbound: {binary}")
    verify_descriptor_reads_probe(value)
    if minimum_cut_bytes is not None:
        expected_cut = json.loads(minimum_cut_bytes)
        probe = value["stored_state_probe"]
        if (
            value.get("runtime_profile") != "formal_full_state_sync_read_only_v1"
            or value.get("minimum_full_sync_cut_sha256") != minimum_cut_sha256
            or probe.get("history_head_verified") is not True
            or probe.get("minimum_cut_verified") is not True
            or probe.get("minimum_cut") != expected_cut
            or not full_sync_cut_is_at_or_after(expected_cut, probe.get("current_cut"))
        ):
            fail("full-sync compatibility probe did not prove the exact staged cut in current history")
    return value


def verify_descriptor_reads_probe(value: dict[str, object]) -> None:
    profile = value.get("runtime_profile")
    formats = value.get("software_formats")
    probe = value.get("stored_state_probe")
    if not isinstance(formats, dict) or not isinstance(probe, dict):
        fail("runtime compatibility descriptor omitted formats or stored-state probe")
    if profile == "compatibility_observer_v1":
        if probe.get("state") == "verified":
            world_version = probe.get("world_format_version")
            read_world = formats.get("world_persistence", {}).get("read_versions") if isinstance(formats.get("world_persistence"), dict) else None
            if not isinstance(world_version, int) or not isinstance(read_world, list) or world_version not in read_world:
                fail("runtime decoder does not support the persisted World format")
            head = probe.get("execution_head")
            history_verified = probe.get("execution_history_verified")
            if history_verified is not isinstance(head, dict):
                fail("runtime probe history-head proof is inconsistent")
            if isinstance(head, dict):
                record_schema = head.get("schema_version")
                read_records = formats.get("execution_bridge_record", {}).get("read_versions") if isinstance(formats.get("execution_bridge_record"), dict) else None
                if not isinstance(record_schema, int) or not isinstance(read_records, list) or record_schema not in read_records:
                    fail("runtime decoder does not support the persisted execution record schema")
    elif profile == "formal_public_observer_v1":
        local_status = formats.get("local_status")
        if probe.get("status_loaded") is True and (not isinstance(local_status, dict) or local_status.get("api_version") != "oasis7.formal-public-observer-status/v1"):
            fail("observer runtime does not support the persisted status format")
    elif profile == "formal_full_state_sync_read_only_v1":
        local_status = formats.get("local_status")
        if probe.get("status_loaded") is True and (not isinstance(local_status, dict) or local_status.get("api_version") != "oasis7.formal-full-sync-status/v1"):
            fail("full-sync runtime does not support the persisted status format")
        if probe.get("history_head_verified") is not True:
            fail("full-sync runtime has not proved its persisted history closure")
    else:
        fail("runtime compatibility descriptor contains an unsupported profile")


def full_sync_cut_is_at_or_after(minimum: object, current: object) -> bool:
    if not isinstance(minimum, dict) or not isinstance(current, dict):
        return False
    if minimum.get("world") != current.get("world") or minimum.get("chain_id") != current.get("chain_id"):
        return False
    minimum_position = minimum.get("position")
    current_position = current.get("position")
    if not isinstance(minimum_position, int) or not isinstance(current_position, int):
        return False
    return current_position > minimum_position or (current_position == minimum_position and current == minimum)


def validate_runtime_compatibility_plan(
    plan_path: Path,
    expected_sha256: str,
    operation_id: str,
    profile: str,
    current_binding: dict[str, object],
    lifecycle_path: Path,
    lifecycle_config_digest: str,
    candidate_runtime: Path,
    candidate_runtime_sha256: str,
) -> dict[str, object]:
    if not isinstance(current_binding, dict):
        fail("operation package stage has no exact current-release binding")
    real_regular_file(plan_path, "runtime compatibility plan")
    if plan_path.resolve(strict=True) != plan_path or digest_file(plan_path) != expected_sha256:
        fail("runtime compatibility plan path or immutable digest changed")
    try:
        plan_bytes = plan_path.read_bytes()
        plan = json.loads(plan_bytes)
    except Exception as exc:
        fail(f"runtime compatibility plan is unreadable: {exc}")
    if not isinstance(plan, dict) or (
        plan.get("api_version") != "oasis7.package-runtime-compatibility-plan/v1"
        or plan.get("profile") != profile
        or plan.get("operation_id") != operation_id
        or plan.get("config_path") != str(lifecycle_path / "node.json")
        or plan.get("data_dir") != str(lifecycle_path)
        or plan.get("candidate_runtime_sha256") != candidate_runtime_sha256
    ):
        fail("runtime compatibility plan does not bind the exact operation, config, data, and candidate")
    minimum_cut_bytes, minimum_cut_sha256 = minimum_cut_binding(plan)
    prior = current_binding.get("prior_runtime")
    if not isinstance(prior, dict) or (
        plan.get("prior_runtime_path") != prior.get("runtime_path")
        or plan.get("prior_runtime_sha256") != prior.get("runtime_sha256")
    ):
        fail("runtime compatibility plan prior executable differs from exact current release provenance")
    prior_path = Path(str(plan["prior_runtime_path"]))
    candidate_path = Path(str(plan["candidate_runtime_path"]))
    real_regular_file(prior_path, "prior compatibility runtime")
    real_regular_file(candidate_path, "candidate compatibility runtime")
    if digest_file(prior_path) != plan["prior_runtime_sha256"] or digest_file(candidate_path) != candidate_runtime_sha256 or digest_file(candidate_runtime) != candidate_runtime_sha256:
        fail("prior or candidate executable changed after compatibility planning")
    prior_descriptor = plan.get("prior_descriptor")
    candidate_descriptor = plan.get("candidate_descriptor")
    if not isinstance(prior_descriptor, dict) or not isinstance(candidate_descriptor, dict):
        fail("runtime compatibility plan omitted executable descriptors")
    expected_config_digest = lifecycle_config_digest
    if not isinstance(expected_config_digest, str) or len(expected_config_digest) != 64 or any(
        descriptor.get("config_digest") != expected_config_digest
        or descriptor.get("config_path") != str(lifecycle_path / "node.json")
        for descriptor in (prior_descriptor, candidate_descriptor)
    ):
        fail("runtime compatibility descriptors are not bound to the staged config digest")
    actual_prior = run_compatibility_descriptor(prior_path, lifecycle_path / "node.json", lifecycle_path, str(plan["prior_runtime_sha256"]))
    actual_candidate = run_compatibility_descriptor(
        candidate_path,
        lifecycle_path / "node.json",
        lifecycle_path,
        candidate_runtime_sha256,
        minimum_cut_bytes,
        minimum_cut_sha256,
    )
    for actual, staged in ((actual_prior, prior_descriptor), (actual_candidate, candidate_descriptor)):
        if any(actual.get(key) != staged.get(key) for key in ("runtime_profile", "world_id", "chain_id", "software_formats", "config_path", "config_digest", "config_sha256")):
            fail("executable descriptor differs from the staged compatibility plan")
    if actual_prior.get("runtime_profile") != actual_candidate.get("runtime_profile") or actual_prior.get("world_id") != actual_candidate.get("world_id") or actual_prior.get("chain_id") != actual_candidate.get("chain_id"):
        fail("prior and candidate executables do not describe the same runtime/world identity")
    if Path(str(plan_path.parent)) != lifecycle_path / "package-runtime-compatibility":
        fail("runtime compatibility plan is outside its private lifecycle transaction directory")
    return plan


def minimum_cut_binding(plan: dict[str, object]) -> tuple[bytes | None, str | None]:
    cut = plan.get("minimum_full_sync_cut")
    encoded = plan.get("minimum_full_sync_cut_bytes")
    sha256 = plan.get("minimum_full_sync_cut_sha256")
    if cut is None and encoded is None and sha256 is None:
        prior_descriptor = plan.get("prior_descriptor")
        if isinstance(prior_descriptor, dict) and prior_descriptor.get("runtime_profile") == "formal_full_state_sync_read_only_v1":
            fail("full-sync compatibility plan omitted its staged minimum cut")
        return None, None
    if not isinstance(cut, dict) or not isinstance(encoded, str) or not isinstance(sha256, str):
        fail("full-sync compatibility plan has incomplete staged minimum-cut evidence")
    raw = encoded.encode("utf-8")
    if digest_bytes(raw) != sha256:
        fail("full-sync staged minimum-cut bytes differ from their immutable digest")
    try:
        decoded = json.loads(raw)
    except Exception as exc:
        fail(f"full-sync staged minimum-cut bytes are invalid JSON: {exc}")
    if decoded != cut:
        fail("full-sync staged minimum-cut bytes differ from the typed cut in the immutable plan")
    return raw, sha256


def revalidate_runtime_compatibility(request: dict[str, object], executable_role: str) -> dict[str, object]:
    plan = request.get("runtime_compatibility_plan")
    lifecycle = request.get("lifecycle_owner")
    if not isinstance(plan, dict) or not isinstance(lifecycle, dict):
        fail("operation package has no exact executable compatibility plan")
    plan_path = Path(str(request.get("runtime_compatibility_plan_path", "")))
    plan_sha256 = str(request.get("runtime_compatibility_plan_sha256", ""))
    real_regular_file(plan_path, "runtime compatibility plan")
    if plan_path.resolve(strict=True) != plan_path or digest_file(plan_path) != plan_sha256:
        fail("runtime compatibility plan changed after durable stage")
    try:
        stored_plan = json.loads(plan_path.read_bytes())
    except Exception as exc:
        fail(f"runtime compatibility plan is unreadable: {exc}")
    if stored_plan != plan or plan.get("operation_id") != request.get("operation_id"):
        fail("runtime compatibility plan differs from the exact durable package request")
    minimum_cut_bytes, minimum_cut_sha256 = minimum_cut_binding(plan)
    owner_data_dir = Path(str(lifecycle.get("data_dir"))).resolve(strict=True)
    if plan_path.parent != owner_data_dir / "package-runtime-compatibility":
        fail("runtime compatibility plan moved outside its exact private data directory")
    if executable_role == "prior":
        binary = Path(str(plan.get("prior_runtime_path", "")))
        expected_sha = str(plan.get("prior_runtime_sha256", ""))
        descriptor_key = "prior_descriptor"
    elif executable_role == "candidate":
        binary = Path(str(request["target"]["node_root"])) / "package-upgrade-operations" / str(request["operation_id"]) / "prepared" / "oasis7-linux-x64" / "bin" / "oasis7_chain_runtime"
        expected_sha = str(request["inputs"].get("runtime_sha256", ""))
        descriptor_key = "candidate_descriptor"
    else:
        fail("unknown executable compatibility role")
    real_regular_file(binary, f"{executable_role} compatibility runtime")
    if digest_file(binary) != expected_sha:
        fail(f"{executable_role} runtime bytes differ from the immutable package operation")
    actual = run_compatibility_descriptor(
        binary,
        Path(str(plan.get("config_path", ""))),
        owner_data_dir,
        expected_sha,
        minimum_cut_bytes,
        minimum_cut_sha256,
    )
    owner_config_digest = lifecycle.get("config_digest")
    if not isinstance(owner_config_digest, str) or len(owner_config_digest) != 64:
        fail("package operation omitted the exact prior lifecycle config digest")
    prior_descriptor = plan.get("prior_descriptor")
    if not isinstance(prior_descriptor, dict):
        fail("package compatibility plan omitted the prior config binding")
    allowed_config_states = {
        (owner_config_digest, prior_descriptor.get("config_sha256")),
    }
    if executable_role == "prior":
        candidate_plan = request.get("candidate_chain_plan")
        if isinstance(candidate_plan, dict):
            allowed_config_states.add((
                candidate_plan.get("candidate_node_config_digest"),
                candidate_plan.get("candidate_node_config_bytes_sha256"),
            ))
    else:
        # Candidate software is admitted before promote mutates either config
        # file. It must therefore be probed against the exact prior config.
        allowed_config_states = {
            (owner_config_digest, prior_descriptor.get("config_sha256")),
        }
    actual_config_state = (actual.get("config_digest"), actual.get("config_sha256"))
    if actual_config_state not in allowed_config_states:
        fail(f"{executable_role} compatibility probe read a config outside the exact operation config state")
    staged_descriptor = plan.get(descriptor_key)
    if not isinstance(staged_descriptor, dict) or any(
        actual.get(key) != staged_descriptor.get(key)
        for key in ("runtime_profile", "world_id", "chain_id", "software_formats")
    ):
        fail(f"{executable_role} runtime compatibility descriptor differs from immutable stage")
    return actual


def tree_digest(root: Path) -> str:
    if root.is_symlink() or not root.is_dir():
        fail(f"prepared package is not a real directory: {root}")
    entries: list[dict[str, object]] = []
    for directory, names, filenames in os.walk(root, topdown=True, followlinks=False):
        parent = Path(directory)
        names.sort()
        filenames.sort()
        for name in list(names):
            path = parent / name
            metadata = path.lstat()
            if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
                fail(f"prepared package contains a non-directory or symlink: {path}")
            entries.append({"path": path.relative_to(root).as_posix() + "/", "kind": "directory", "mode": stat.S_IMODE(metadata.st_mode)})
        for name in filenames:
            path = parent / name
            metadata = path.lstat()
            if not stat.S_ISREG(metadata.st_mode):
                fail(f"prepared package contains a non-regular file: {path}")
            entries.append({"path": path.relative_to(root).as_posix(), "kind": "file", "mode": stat.S_IMODE(metadata.st_mode), "size": metadata.st_size, "sha256": digest_file(path)})
    entries.sort(key=lambda entry: str(entry["path"]))
    return digest_bytes(canonical(entries))


def file_state(path: Path, root: Path) -> dict[str, object]:
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return {"path": path.relative_to(root).as_posix(), "present": False}
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"live package preimage contains a non-regular file: {path}")
    return {
        "path": path.relative_to(root).as_posix(),
        "present": True,
        "sha256": digest_file(path),
        "size": metadata.st_size,
        "uid": metadata.st_uid,
        "gid": metadata.st_gid,
        "mode": stat.S_IMODE(metadata.st_mode),
    }


def live_preimage(
    root: Path,
    *,
    allow_empty_config: bool = False,
    profile: str | None = None,
) -> dict[str, object]:
    root = root.resolve(strict=True)
    current = root / "current"
    if profile is not None:
        if profile not in ("public_testnet_linux_package_v1", "local_native_package_v1"):
            fail("unsupported package deployment profile")
        selected = read_current_release_snapshot(root, profile)
        current_identity = selected["current"]
        runtime_identity = selected["current_runtime"]
    else:
        try:
            current_metadata = current.lstat()
        except FileNotFoundError:
            current_identity = {"kind": "absent"}
        else:
            if stat.S_ISLNK(current_metadata.st_mode):
                current_identity = {
                    "kind": "symlink",
                    "target": os.readlink(current),
                    "resolved": str(current.resolve(strict=False)),
                }
            elif stat.S_ISDIR(current_metadata.st_mode):
                current_identity = {
                    "kind": "directory",
                    "resolved": str(current.resolve(strict=False)),
                    "uid": current_metadata.st_uid,
                    "gid": current_metadata.st_gid,
                    "mode": stat.S_IMODE(current_metadata.st_mode),
                }
            elif stat.S_ISREG(current_metadata.st_mode):
                current_identity = {
                    "kind": "file",
                    "resolved": str(current.resolve(strict=False)),
                    "sha256": digest_file(current),
                    "uid": current_metadata.st_uid,
                    "gid": current_metadata.st_gid,
                    "mode": stat.S_IMODE(current_metadata.st_mode),
                }
            else:
                fail(f"unsupported live current identity: {current}")

        runtime_path = root / "current" / "bin" / "oasis7_chain_runtime"
        runtime_real = runtime_path.resolve(strict=False)
        try:
            runtime_metadata = runtime_real.lstat()
        except FileNotFoundError:
            runtime_identity = {"present": False}
        else:
            if not stat.S_ISREG(runtime_metadata.st_mode):
                fail(f"current runtime is not a regular file: {runtime_real}")
            runtime_identity = {
                "present": True,
                "resolved": str(runtime_real),
                "sha256": digest_file(runtime_real),
                "size": runtime_metadata.st_size,
            }

    bundle_name = "public-testnet-governed-bootstrap-bundle-2026-06-06.json"
    bundles = sorted((root / "config").rglob(bundle_name))
    if not bundles and not allow_empty_config:
        fail(f"no governed bootstrap bundle found under {root / 'config'}")
    config_files = [file_state(path, root) for path in bundles]
    if any(entry.get("present") is not True for entry in config_files):
        fail("governed bootstrap bundle is not a regular file")
    config_digest = digest_bytes(canonical(config_files))
    metadata_files = [
        file_state(root / "CURRENT_VERSION", root),
        file_state(root / "DEPLOYED_BUILDINFO", root),
    ]
    return {
        "current": current_identity,
        "current_runtime": runtime_identity,
        "config_files": config_files,
        "config_digest": config_digest,
        "deployment_metadata": metadata_files,
    }


def validate_operation_current(root: Path, profile: str) -> dict[str, object]:
    if profile not in ("public_testnet_linux_package_v1", "local_native_package_v1"):
        fail("unsupported package deployment profile")
    root = root.resolve(strict=True)
    selected = read_current_release_snapshot(root, profile)
    return {"current": selected["current"], "prior_runtime": selected["prior_runtime"]}


def real_directory(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"prior package {description} directory is unavailable: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
        fail(f"prior package {description} directory contains a symlink or is not a directory")


def real_regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"prior package {description} is unavailable: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"prior package {description} contains a symlink or is not a regular file")


def provenance_fields(path: Path) -> dict[str, str]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        fail(f"cannot read prior package BUILDINFO: {error}")
    fields: dict[str, str] = {}
    for line_number, line in enumerate(lines, 1):
        if "=" not in line:
            fail(f"prior package BUILDINFO line {line_number} is malformed")
        name, value = line.split("=", 1)
        if not name.strip() or name in fields:
            fail("prior package BUILDINFO has an empty or duplicate field")
        fields[name] = value
    return fields


def check_private_directory(path: Path, *, create: bool = False) -> None:
    if create:
        path.mkdir(mode=0o700, parents=True, exist_ok=True)
    metadata = path.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        fail(f"operation transaction path is not a real directory: {path}")
    if metadata.st_uid != os.geteuid() or stat.S_IMODE(metadata.st_mode) & 0o077:
        fail(f"operation transaction directory must be owner-private: {path}")


def write_immutable(path: Path, payload: bytes) -> None:
    with path.open("xb") as handle:
        handle.write(payload)
        handle.flush()
        os.fchmod(handle.fileno(), 0o600)
        os.fsync(handle.fileno())


def journal_transaction_phase(
    transaction_dir: Path,
    phase: str,
    promoted_current: str | None = None,
) -> None:
    manifest_path = transaction_dir / "transaction.json"
    manifest = strict_json(manifest_path)
    if not isinstance(manifest, dict):
        fail("package rollback transaction manifest is invalid")
    manifest["phase"] = phase
    if promoted_current is not None:
        manifest["promoted_current"] = {
            "kind": "symlink",
            "target": promoted_current,
            "resolved": str(Path(promoted_current).resolve(strict=False)),
        }
    temporary = manifest_path.with_name(f".{manifest_path.name}.{os.getpid()}.tmp")
    with temporary.open("xb") as handle:
        handle.write((json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8"))
        os.fchmod(handle.fileno(), 0o600)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, manifest_path)
    directory_fd = os.open(transaction_dir, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def restore_operation_snapshot(transaction_dir: Path) -> None:
    manifest_path = transaction_dir / "transaction.json"
    manifest = strict_json(manifest_path)
    if not isinstance(manifest, dict) or manifest.get("schema_version") != "oasis7.package_upgrade_rollback.v1":
        fail("package rollback snapshot schema is invalid")
    root = Path(str(manifest.get("node_root", ""))).resolve(strict=True)
    current_state = manifest.get("current")
    promoted_current = manifest.get("promoted_current")
    if not isinstance(current_state, dict) or current_state.get("kind") != "symlink":
        fail("package rollback requires an original immutable release symlink")
    if not isinstance(promoted_current, dict) or promoted_current.get("kind") != "symlink":
        fail("package rollback snapshot has no exact promoted current identity")
    current_path = root / "current"

    def identity(path: Path) -> dict[str, object]:
        if path.is_symlink():
            return {"kind": "symlink", "target": os.readlink(path), "resolved": str(path.resolve(strict=False))}
        return {"kind": "absent"}

    phase = str(manifest.get("phase", ""))
    actual_current = identity(current_path)
    expected_current = promoted_current if phase != "snapshotted" else current_state
    current_is_snapshot = actual_current == current_state
    current_is_promoted = (
        actual_current.get("kind") == "symlink"
        and actual_current.get("resolved") == promoted_current.get("resolved")
    )
    if phase in ("rollback_started", "rolled_back"):
        if not (current_is_snapshot or current_is_promoted):
            fail("package rollback current identity drifted during interrupted restore")
    elif actual_current != expected_current and not current_is_promoted:
        fail("package rollback current identity differs from the exact promoted release")

    lifecycle_files = manifest.get("lifecycle_files", [])
    if not isinstance(lifecycle_files, list):
        fail("package rollback lifecycle config snapshot is invalid")
    for entry in lifecycle_files:
        if not isinstance(entry, dict):
            fail("package rollback lifecycle config entry is invalid")
        path = Path(str(entry.get("path", "")))
        backup_relative = Path(str(entry.get("backup", "")))
        if not path.is_absolute() or backup_relative.is_absolute() or ".." in backup_relative.parts:
            fail("package rollback lifecycle config path is unsafe")
        real_regular_file(path, "lifecycle config")
        actual_digest = digest_file(path)
        allowed = {entry.get("sha256"), entry.get("candidate_sha256")}
        if actual_digest not in allowed:
            fail(f"package rollback lifecycle config left the old/candidate transaction states: {path}")
        if phase not in ("rollback_started", "rolled_back") and actual_digest != entry.get("candidate_sha256"):
            fail(f"package rollback lifecycle config does not match the exact promoted candidate: {path}")

    if manifest.get("prior_runtime") is not None:
        prior_runtime = restored_runtime_identity(manifest, root, manifest)
        if prior_runtime != manifest.get("prior_runtime"):
            fail("prior release runtime/BUILDINFO provenance drifted from the rollback snapshot")

    expected_files = manifest.get("files")
    if not isinstance(expected_files, list):
        fail("package rollback snapshot file list is invalid")
    for entry in expected_files:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            fail("package rollback snapshot file entry is invalid")
        relative = Path(str(entry["path"]))
        if relative.is_absolute() or ".." in relative.parts or not relative.parts:
            fail("package rollback snapshot contains an unsafe metadata path")
        if entry.get("present") is not True:
            if entry.get("present") is not False or "backup" in entry:
                fail("package rollback absent-file snapshot entry is invalid")
            continue
        backup_relative = Path(str(entry.get("backup", "")))
        if backup_relative.is_absolute() or ".." in backup_relative.parts or not backup_relative.parts:
            fail("package rollback snapshot contains an unsafe backup path")
        backup = transaction_dir / backup_relative
        backup_parent = transaction_dir
        for component in backup_relative.parts[:-1]:
            backup_parent = backup_parent / component
            try:
                parent_metadata = backup_parent.lstat()
            except OSError as error:
                fail(f"package rollback snapshot directory is missing: {backup_parent}: {error}")
            if not stat.S_ISDIR(parent_metadata.st_mode) or stat.S_ISLNK(parent_metadata.st_mode):
                fail(f"package rollback snapshot path contains a non-directory: {backup_parent}")
        try:
            backup_metadata = backup.lstat()
        except OSError as error:
            fail(f"package rollback snapshot file is missing: {backup}: {error}")
        if not stat.S_ISREG(backup_metadata.st_mode) or stat.S_ISLNK(backup_metadata.st_mode):
            fail(f"package rollback snapshot file is not regular: {backup}")
        if (
            not isinstance(entry.get("sha256"), str)
            or digest_file(backup) != entry["sha256"]
            or backup_metadata.st_size != entry.get("size")
            or not isinstance(entry.get("uid"), int)
            or not isinstance(entry.get("gid"), int)
            or not isinstance(entry.get("mode"), int)
        ):
            fail(f"package rollback snapshot file differs from its original preimage: {backup}")

    manifest["phase"] = "rollback_started"
    temporary = manifest_path.with_name(f".{manifest_path.name}.{os.getpid()}.{os.urandom(4).hex()}.tmp")
    with temporary.open("xb") as handle:
        handle.write((json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8"))
        os.fchmod(handle.fileno(), 0o600)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, manifest_path)
    directory_fd = os.open(transaction_dir, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)

    for entry in expected_files:
        relative = Path(str(entry["path"]))
        if relative.is_absolute() or ".." in relative.parts:
            fail("package rollback snapshot contains an unsafe metadata path")
        destination = root / relative
        if entry.get("present") is not True:
            if destination.is_symlink() or destination.is_file():
                destination.unlink()
                parent_fd = os.open(destination.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
                try:
                    os.fsync(parent_fd)
                finally:
                    os.close(parent_fd)
            elif destination.exists():
                fail(f"package rollback refuses to remove a non-file target: {destination}")
            continue
        backup = transaction_dir / str(entry.get("backup", ""))
        if backup.is_symlink() or not backup.is_file():
            fail(f"package rollback snapshot file is missing: {backup}")
        destination.parent.mkdir(parents=True, exist_ok=True)
        temp = destination.with_name(f".{destination.name}.rollback-{os.getpid()}-{os.urandom(4).hex()}.tmp")
        shutil.copy2(backup, temp)
        os.chown(temp, int(entry["uid"]), int(entry["gid"]))
        os.chmod(temp, int(entry["mode"]))
        with temp.open("rb") as handle:
            os.fsync(handle.fileno())
        os.replace(temp, destination)
        parent_fd = os.open(destination.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)

    for entry in lifecycle_files:
        path = Path(str(entry["path"]))
        backup_relative = Path(str(entry["backup"]))
        backup_parent = transaction_dir
        for component in backup_relative.parts[:-1]:
            backup_parent = backup_parent / component
            metadata = backup_parent.lstat()
            if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
                fail(f"package rollback lifecycle snapshot contains an unsafe directory: {backup_parent}")
        backup = transaction_dir / backup_relative
        metadata = backup.lstat()
        if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
            fail(f"package rollback lifecycle snapshot is not a regular file: {backup}")
        if digest_file(backup) != entry.get("sha256") or metadata.st_size != entry.get("size"):
            fail(f"package rollback lifecycle snapshot bytes differ from their exact preimage: {backup}")
        temporary = path.with_name(f".{path.name}.rollback-{os.getpid()}-{os.urandom(4).hex()}.tmp")
        shutil.copy2(backup, temporary)
        os.chown(temporary, int(entry["uid"]), int(entry["gid"]))
        os.chmod(temporary, int(entry["mode"]))
        with temporary.open("rb") as handle:
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        parent_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)

    if not current_path.is_symlink() and (current_path.exists() or current_path.is_file() or current_path.is_dir()):
        fail("package rollback refuses to replace a non-symlink current path")
    temporary_link = current_path.with_name(f".{current_path.name}.rollback-{os.getpid()}-{os.urandom(4).hex()}.tmp")
    os.symlink(str(current_state["target"]), temporary_link)
    # Replacing the selected symlink in one rename avoids a SIGKILL window in
    # which neither the promoted nor restored runtime is selected.
    os.replace(temporary_link, current_path)
    parent_fd = os.open(current_path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(parent_fd)
    finally:
        os.close(parent_fd)

    manifest["phase"] = "rolled_back"
    temporary = manifest_path.with_name(f".{manifest_path.name}.{os.getpid()}.{os.urandom(4).hex()}.tmp")
    with temporary.open("xb") as handle:
        handle.write((json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8"))
        os.fchmod(handle.fileno(), 0o600)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, manifest_path)
    directory_fd = os.open(transaction_dir, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def fsync_tree(root: Path) -> None:
    for directory, names, filenames in os.walk(root, topdown=False, followlinks=False):
        path = Path(directory)
        for name in filenames:
            file_path = path / name
            if file_path.is_symlink() or not file_path.is_file():
                fail(f"operation transaction contains a non-regular file: {file_path}")
            with file_path.open("rb") as handle:
                os.fsync(handle.fileno())
        for name in names:
            child = path / name
            if child.is_symlink() or not child.is_dir():
                fail(f"operation transaction contains a non-directory: {child}")
        directory_fd = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)


def request_for(args: list[str]) -> dict[str, object]:
    if len(args) not in (11, 17, 19, 21):
        fail("invalid package stage request arguments")
    (
        root_text,
        operation_id,
        action_id,
        expected_instance,
        expected_generation,
        package_version,
        commit,
        run_id,
        artifact_ref,
        artifacts_text,
        prepared_text,
    ) = args[:11]
    profile, operation_record_digest, expected_incarnation, expected_config_digest, expected_instance_action_id, lifecycle_data_dir = (
        args[11:17] if len(args) >= 17 else ("", "", "", "", "", "")
    )
    candidate_plan_path, candidate_plan_sha256 = args[17:19] if len(args) in (19, 21) else ("", "")
    runtime_compatibility_plan_path, runtime_compatibility_plan_sha256 = (
        args[19:21] if len(args) == 21 else ("", "")
    )
    root = Path(root_text).resolve(strict=True)
    current_binding = validate_operation_current(root, profile) if profile else None
    artifacts = Path(artifacts_text)
    prepared = Path(prepared_text)
    package_path = artifacts / "package.deb"
    ops_path = artifacts / "ops-tools.tar.gz"
    buildinfo_path = prepared / "BUILDINFO"
    runtime_path = prepared / "bin" / "oasis7_chain_runtime"
    ops_manifest = artifacts / "ops-tools.manifest"
    try:
        buildinfo_lines = buildinfo_path.read_text(encoding="utf-8").splitlines()
    except Exception as exc:
        fail(f"cannot read verified BUILDINFO: {exc}")
    buildinfo: dict[str, str] = {}
    for line in buildinfo_lines:
        if "=" in line:
            key, value = line.split("=", 1)
            buildinfo[key] = value
    preimage = live_preimage(
        root,
        allow_empty_config=profile == "local_native_package_v1",
        profile=profile or None,
    )
    if current_binding is not None and preimage.get("current") != current_binding.get("current"):
        fail("live current changed while capturing the immutable package preimage")
    request = {
        "schema_version": (
            "oasis7.package_upgrade_operation_request.v2"
            if profile else "oasis7.package_upgrade_operation_request.v1"
        ),
        "operation_id": operation_id,
        "action_id": action_id,
        "action": "package.stage",
        "target": {
            "node_root": str(root),
            "expected_instance_id": expected_instance,
            "expected_generation": int(expected_generation),
        },
        "parameters": {
            "package_version": package_version,
            "commit": commit,
            "run_id": run_id,
            "artifact_ref": artifact_ref,
        },
        "inputs": {
            "package_deb_sha256": digest_file(package_path),
            "ops_tools_tar_sha256": digest_file(ops_path),
            "ops_tools_manifest_sha256": digest_file(ops_manifest),
            "buildinfo_sha256": digest_file(buildinfo_path),
            "buildinfo": buildinfo,
            "runtime_sha256": digest_file(runtime_path),
            "prepared_bundle_sha256": tree_digest(prepared),
        },
        "preimage": preimage,
    }
    if profile:
        request["prior_runtime"] = current_binding["prior_runtime"]
        if len(operation_record_digest) != 64 or any(c not in "0123456789abcdef" for c in operation_record_digest):
            fail("operation record digest must be canonical SHA256")
        if profile not in ("public_testnet_linux_package_v1", "local_native_package_v1"):
            fail("unsupported package deployment profile")
        if not expected_incarnation or not expected_config_digest or not expected_instance_action_id or not lifecycle_data_dir:
            fail("package stage exact lifecycle owner binding is incomplete")
        lifecycle_path = Path(lifecycle_data_dir).resolve(strict=True)
        check_private_directory(lifecycle_path)
        instance = strict_json(lifecycle_path / "instance.json")
        if not isinstance(instance, dict) or (
            instance.get("api_version") != "oasis7.local-node-instance/v1"
            or instance.get("incarnation") != expected_incarnation
            or instance.get("generation") != int(expected_generation)
            or instance.get("config_digest") != expected_config_digest
            or instance.get("action_id") != expected_instance_action_id
            or instance.get("state") != "running"
            or instance.get("runtime_pid") is None
        ):
            fail("package stage requires the exact live lifecycle owner on this host")
        request["profile"] = profile
        request["operation_record_digest"] = operation_record_digest
        request["lifecycle_owner"] = {
            "data_dir": str(lifecycle_path),
            "incarnation": expected_incarnation,
            "generation": int(expected_generation),
            "config_digest": expected_config_digest,
            "instance_action_id": expected_instance_action_id,
        }
        if not runtime_compatibility_plan_path or len(runtime_compatibility_plan_sha256) != 64:
            fail("operation-mode package staging requires an immutable executable compatibility plan")
        compatibility_path = Path(runtime_compatibility_plan_path)
        compatibility_plan = validate_runtime_compatibility_plan(
            compatibility_path,
            runtime_compatibility_plan_sha256,
            operation_id,
            profile,
            current_binding,
            lifecycle_path,
            expected_config_digest,
            runtime_path,
            digest_file(runtime_path),
        )
        request["runtime_compatibility_plan"] = compatibility_plan
        request["runtime_compatibility_plan_path"] = str(compatibility_path)
        request["runtime_compatibility_plan_sha256"] = runtime_compatibility_plan_sha256
        if candidate_plan_path or candidate_plan_sha256:
            if profile != "local_native_package_v1" or not candidate_plan_path or len(candidate_plan_sha256) != 64:
                fail("candidate-chain plan is supported only as a complete local-native binding")
            candidate_plan_path = Path(candidate_plan_path)
            real_regular_file(candidate_plan_path, "candidate-chain plan")
            if candidate_plan_path.resolve(strict=True) != candidate_plan_path:
                fail("candidate-chain plan path must be canonical and contain no symlink")
            if digest_file(candidate_plan_path) != candidate_plan_sha256:
                fail("candidate-chain plan digest changed before helper admission")
            try:
                plan_bytes = candidate_plan_path.read_bytes()
                candidate_plan_json = plan_bytes.decode("utf-8")
                candidate_plan = json.loads(candidate_plan_json)
            except Exception as exc:
                fail(f"candidate-chain plan is unreadable: {exc}")
            validate_candidate_chain_plan(candidate_plan, lifecycle_path, check_preimage=True)
            request["candidate_chain_plan"] = candidate_plan
            request["candidate_chain_plan_sha256"] = candidate_plan_sha256
            request["candidate_chain_plan_json"] = candidate_plan_json
    return request


def validate_candidate_chain_plan(
    plan: object,
    lifecycle_path: Path,
    *,
    check_preimage: bool,
) -> None:
    if not isinstance(plan, dict) or plan.get("api_version") != "oasis7.local-native-candidate-chain-plan/v1":
        fail("candidate-chain plan schema is unsupported")
    input_pin = plan.get("input")
    if not isinstance(input_pin, dict) or input_pin.get("api_version") != "oasis7.operator-pinned-candidate-chain-config/v1":
        fail("candidate-chain plan has no exact operator input pin")
    input_path = Path(str(input_pin.get("path", "")))
    input_bytes = plan.get("input_bytes")
    if not input_path.is_absolute() or not isinstance(input_bytes, str):
        fail("candidate-chain input path or bytes are invalid")
    if digest_bytes(input_bytes.encode("utf-8")) != input_pin.get("sha256"):
        fail("candidate-chain input bytes do not match the operator SHA256")
    if not isinstance(plan.get("candidate_material"), list) or len(plan["candidate_material"]) != 6:
        fail("candidate-chain plan must freeze the six pinned chain materials")
    roles = {"chain_config", "manifest", "genesis", "registry", "release_bundle", "bootstrap_peers"}
    materials: dict[str, dict[str, object]] = {}
    total_size = 0
    for item in plan["candidate_material"]:
        if not isinstance(item, dict) or item.get("role") not in roles or item["role"] in materials:
            fail("candidate-chain plan contains an unsupported or duplicate material role")
        path = Path(str(item.get("path", "")))
        content = item.get("bytes")
        if not path.is_absolute() or not isinstance(content, str):
            fail("candidate-chain material path or bytes are invalid")
        raw = content.encode("utf-8")
        if digest_bytes(raw) != item.get("sha256"):
            fail("candidate-chain material bytes do not match their pinned SHA256")
        total_size += len(raw)
        materials[str(item["role"])] = item
    if set(materials) != roles or total_size > 2 * 1024 * 1024:
        fail("candidate-chain material set or size is invalid")
    if materials["chain_config"].get("path") != str(input_path) or materials["chain_config"].get("sha256") != input_pin.get("sha256"):
        fail("candidate-chain config material differs from the exact operator pin")
    if materials["chain_config"].get("bytes") != input_bytes:
        fail("candidate-chain config material bytes differ from the captured operator input")
    candidate_anchor = candidate_anchor_from_captured_input(
        input_path,
        input_bytes,
        materials,
    )
    if plan.get("candidate_anchor") != candidate_anchor:
        fail("candidate-chain anchor was not derived from the exact frozen config and material bytes")

    prior_config = plan.get("prior_node_config")
    prior_binding = plan.get("prior_node_binding")
    candidate_config_path = Path(str(plan.get("candidate_node_config_path", "")))
    candidate_binding_path = Path(str(plan.get("candidate_node_binding_path", "")))
    candidate_config_bytes = plan.get("candidate_node_config_bytes")
    candidate_binding_bytes = plan.get("candidate_node_binding_bytes")
    if not isinstance(prior_config, dict) or not isinstance(prior_binding, dict):
        fail("candidate-chain plan omits exact old node config/binding snapshots")
    if not candidate_config_path.is_absolute() or not candidate_binding_path.is_absolute():
        fail("candidate lifecycle config paths must be absolute")
    if candidate_binding_path != lifecycle_path / "node-binding.json":
        fail("candidate node binding path differs from the exact lifecycle data directory")
    if not isinstance(candidate_config_bytes, str) or not isinstance(candidate_binding_bytes, str):
        fail("candidate lifecycle config bytes are missing")
    if digest_bytes(candidate_config_bytes.encode("utf-8")) != plan.get("candidate_node_config_bytes_sha256"):
        fail("candidate node config bytes differ from their plan digest")
    if digest_bytes(candidate_binding_bytes.encode("utf-8")) != plan.get("candidate_node_binding_bytes_sha256"):
        fail("candidate node binding bytes differ from their plan digest")
    if digest_bytes(str(prior_config.get("bytes", "")).encode("utf-8")) != prior_config.get("sha256"):
        fail("prior node config snapshot bytes differ from their digest")
    if digest_bytes(str(prior_binding.get("bytes", "")).encode("utf-8")) != prior_binding.get("sha256"):
        fail("prior node binding snapshot bytes differ from their digest")
    if Path(str(prior_config.get("path", ""))) != candidate_config_path or Path(str(prior_binding.get("path", ""))) != candidate_binding_path:
        fail("candidate config paths do not target the exact old config and binding files")
    if any(not isinstance(plan.get(key), str) or len(str(plan.get(key))) != 64 for key in (
        "prior_node_config_digest", "prior_binding_digest", "candidate_node_config_digest",
        "candidate_binding_digest", "prior_anchor_digest", "candidate_anchor_digest",
        "prior_manifest_sha256", "candidate_manifest_sha256", "prior_bundle_sha256",
        "candidate_bundle_sha256", "candidate_runtime_sha256", "candidate_buildinfo_sha256",
    )):
        fail("candidate-chain plan contains a malformed lifecycle or artifact digest")
    try:
        old_config = json.loads(str(prior_config["bytes"]))
        new_config = json.loads(candidate_config_bytes)
        old_binding = json.loads(str(prior_binding["bytes"]))
        new_binding = json.loads(candidate_binding_bytes)
    except Exception as exc:
        fail(f"candidate-chain lifecycle config JSON is invalid: {exc}")
    if old_binding != old_config.get("identity") or new_binding != new_config.get("identity"):
        fail("candidate node config and node-binding snapshots are not exact identity matches")
    old_projection = dict(old_config)
    new_projection = dict(new_config)
    old_identity = dict(old_projection.get("identity", {}))
    new_identity = dict(new_projection.get("identity", {}))
    old_identity.pop("chain", None)
    new_identity.pop("chain", None)
    old_projection["identity"] = old_identity
    new_projection["identity"] = new_identity
    if old_projection != new_projection or new_config.get("identity", {}).get("chain") != candidate_anchor:
        fail("candidate node config changes node identity or runtime settings beyond its pinned chain anchor")
    if candidate_anchor.get("manifest", {}).get("sha256") != materials["manifest"].get("sha256"):
        fail("candidate chain anchor manifest differs from frozen material")
    for name in ("genesis", "registry", "release_bundle", "bootstrap_peers"):
        pin = candidate_anchor.get(name)
        if not isinstance(pin, dict) or pin.get("sha256") != materials[name].get("sha256") or pin.get("path") != materials[name].get("path"):
            fail(f"candidate chain anchor {name} differs from frozen material")
    if check_preimage:
        for snapshot in (prior_config, prior_binding):
            path = Path(str(snapshot["path"]))
            real_regular_file(path, "prior lifecycle config")
            if path.resolve(strict=True) != path or path.read_bytes().decode("utf-8") != snapshot["bytes"]:
                fail("lifecycle config changed after the candidate preimage was captured")
        for item in materials.values():
            path = Path(str(item["path"]))
            real_regular_file(path, "candidate pinned chain material")
            if path.resolve(strict=True) != path or digest_file(path) != item["sha256"]:
                fail("candidate pinned chain material changed after stage admission")


def candidate_anchor_from_captured_input(
    input_path: Path,
    input_bytes: str,
    materials: dict[str, dict[str, object]],
) -> dict[str, object]:
    def unique_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    def parse_object(raw: str, label: str) -> dict[str, object]:
        try:
            value = json.loads(raw, object_pairs_hook=unique_pairs)
        except Exception as exc:
            fail(f"candidate-chain {label} bytes are invalid JSON: {exc}")
        if not isinstance(value, dict):
            fail(f"candidate-chain {label} must be a JSON object")
        return value

    def resolved_reference(base: Path, value: object, label: str) -> Path:
        if not isinstance(value, str) or not value:
            fail(f"candidate-chain {label} reference is missing")
        path = Path(value)
        if not path.is_absolute():
            path = base / path
        if not path.is_absolute() or ".." in path.parts:
            fail(f"candidate-chain {label} reference is not canonical")
        return Path(os.path.abspath(path))

    config = parse_object(input_bytes, "operator config")
    allowed = {
        "api_version",
        "runtime_profile",
        "world_id",
        "chain_id",
        "genesis_sha256",
        "network_tier_manifest",
        "manifest_sha256",
    }
    if set(config) != allowed or config.get("api_version") != "oasis7.chain-config/v1" or config.get("runtime_profile") != "compatibility_observer_v1":
        fail("candidate-chain frozen config has an unsupported schema or runtime profile")

    manifest_path = resolved_reference(input_path.parent, config.get("network_tier_manifest"), "manifest")
    manifest_material = materials["manifest"]
    if manifest_path != Path(str(manifest_material["path"])) or config.get("manifest_sha256") != manifest_material.get("sha256"):
        fail("candidate-chain frozen config manifest reference differs from the captured manifest material")
    if config.get("genesis_sha256") != materials["genesis"].get("sha256"):
        fail("candidate-chain frozen config genesis digest differs from captured genesis material")

    manifest = parse_object(str(manifest_material["bytes"]), "manifest")
    runtime_refs = manifest.get("runtime_refs")
    registry = manifest.get("deployment_validator_registry")
    if not isinstance(runtime_refs, dict) or not isinstance(registry, dict):
        fail("candidate-chain manifest omits pinned runtime references or deployment registry")
    registry_ref = registry.get("ref")
    if not isinstance(registry_ref, str) or not registry_ref or not Path(registry_ref).name:
        fail("candidate-chain manifest registry reference is invalid")
    references = {
        "genesis": resolved_reference(manifest_path.parent, runtime_refs.get("genesis_ref"), "genesis"),
        "registry": resolved_reference(manifest_path.parent, Path(registry_ref).name, "registry"),
        "release_bundle": resolved_reference(
            manifest_path.parent,
            runtime_refs.get("release_candidate_bundle_ref"),
            "release bundle",
        ),
        "bootstrap_peers": resolved_reference(
            manifest_path.parent,
            runtime_refs.get("bootstrap_peer_ref"),
            "bootstrap peers",
        ),
    }
    anchor: dict[str, object] = {
        "runtime_profile": config["runtime_profile"],
        "world_id": config["world_id"],
        "chain_id": config["chain_id"],
        "manifest": {"path": str(manifest_path), "sha256": manifest_material["sha256"]},
    }
    for role, path in references.items():
        item = materials[role]
        if path != Path(str(item["path"])):
            fail(f"candidate-chain manifest {role} reference differs from captured material")
        anchor[role] = {"path": str(path), "sha256": item["sha256"]}
    return anchor


def verify_candidate_chain_material_files(plan: dict[str, object]) -> None:
    for item in plan["candidate_material"]:
        # The original candidate chain-config is frozen in the immutable
        # plan/request and is not read after admission. Manifest/genesis/
        # registry/peer/bundle refs remain live runtime inputs and must stay
        # byte-identical through promotion, recovery, and rollback.
        if item.get("role") == "chain_config":
            continue
        path = Path(str(item["path"]))
        real_regular_file(path, "candidate pinned chain material")
        if path.resolve(strict=True) != path or digest_file(path) != item["sha256"]:
            fail(f"candidate pinned chain material changed after stage: {path}")


def verify_candidate_lifecycle_state(
    plan: dict[str, object],
    *,
    allow_partial: bool,
    expected: str | None = None,
) -> None:
    snapshots = (
        (plan["prior_node_config"], plan["candidate_node_config_bytes_sha256"]),
        (plan["prior_node_binding"], plan["candidate_node_binding_bytes_sha256"]),
    )
    actual_digests = []
    for snapshot, candidate_digest in snapshots:
        path = Path(str(snapshot["path"]))
        real_regular_file(path, "lifecycle config")
        if path.resolve(strict=True) != path:
            fail("candidate lifecycle config path is not canonical")
        actual = digest_file(path)
        allowed = {str(snapshot["sha256"]), str(candidate_digest)}
        if actual not in allowed:
            fail(f"lifecycle config is outside the exact candidate transaction states: {path}")
        actual_digests.append(actual)
    old = [str(snapshot["sha256"]) for snapshot, _ in snapshots]
    candidate = [str(value) for _, value in snapshots]
    if expected == "old" and actual_digests != old:
        fail("lifecycle config does not match the exact prior candidate-chain preimage")
    if expected == "candidate" and actual_digests != candidate:
        fail("lifecycle config does not match the exact promoted candidate-chain state")
    if not allow_partial and actual_digests not in (old, candidate):
        fail("lifecycle config contains a mixed old/candidate state without a durable intent")


def expected_lifecycle_snapshot(plan: dict[str, object]) -> list[dict[str, object]]:
    entries = []
    for index, (snapshot, candidate_sha256) in enumerate((
        (plan["prior_node_config"], plan["candidate_node_config_bytes_sha256"]),
        (plan["prior_node_binding"], plan["candidate_node_binding_bytes_sha256"]),
    )):
        entries.append({
            "path": str(snapshot["path"]),
            "backup": f"snapshot/lifecycle/{index}.json",
            "sha256": snapshot["sha256"],
            "candidate_sha256": candidate_sha256,
            "size": len(str(snapshot["bytes"]).encode("utf-8")),
            "uid": snapshot["uid"],
            "gid": snapshot["gid"],
            "mode": snapshot["mode"],
        })
    return entries


def validate_lifecycle_snapshot(manifest: dict[str, object], plan: dict[str, object] | None) -> None:
    expected = [] if plan is None else expected_lifecycle_snapshot(plan)
    if manifest.get("lifecycle_files", []) != expected:
        fail("package rollback lifecycle config snapshot differs from the immutable candidate plan")


def expected_receipt(request: dict[str, object], request_digest: str) -> dict[str, object]:
    inputs = request["inputs"]
    target = request["target"]
    parameters = request["parameters"]
    preimage_digest = digest_bytes(canonical(request["preimage"]))
    if request["schema_version"] == "oasis7.package_upgrade_operation_request.v2":
        owner = request["lifecycle_owner"]
        receipt = {
            "schema_version": "oasis7.package_upgrade_operation_receipt.v2",
            "profile": request["profile"],
            "operation_id": request["operation_id"],
            "action_id": request["action_id"],
            "operation_request_digest": request["operation_record_digest"],
            "request_digest": request_digest,
            "node_root": target["node_root"],
            "expected_incarnation": owner["incarnation"],
            "expected_generation": owner["generation"],
            "expected_config_digest": owner["config_digest"],
            "expected_instance_action_id": owner["instance_action_id"],
            "package_version": parameters["package_version"],
            "commit": parameters["commit"],
            "package_deb_sha256": inputs["package_deb_sha256"],
            "ops_tools_tar_sha256": inputs["ops_tools_tar_sha256"],
            "prepared_bundle_sha256": inputs["prepared_bundle_sha256"],
            "current_config_preimage_sha256": preimage_digest,
            "runtime_compatibility_plan_path": request.get("runtime_compatibility_plan_path"),
            "runtime_compatibility_plan_sha256": request.get("runtime_compatibility_plan_sha256"),
        }
        plan = request.get("candidate_chain_plan")
        if plan is not None:
            receipt.update({
                "candidate_chain_plan_sha256": request.get("candidate_chain_plan_sha256"),
                "candidate_chain_config_path": plan["input"]["path"],
                "candidate_chain_config_sha256": plan["input"]["sha256"],
                "prior_node_config_digest": plan["prior_node_config_digest"],
                "prior_binding_digest": plan["prior_binding_digest"],
                "candidate_node_config_digest": plan["candidate_node_config_digest"],
                "candidate_binding_digest": plan["candidate_binding_digest"],
                "prior_anchor_digest": plan["prior_anchor_digest"],
                "candidate_anchor_digest": plan["candidate_anchor_digest"],
                "candidate_manifest_sha256": plan["candidate_manifest_sha256"],
                "candidate_bundle_sha256": plan["candidate_bundle_sha256"],
                "candidate_runtime_sha256": plan["candidate_runtime_sha256"],
                "candidate_buildinfo_sha256": plan["candidate_buildinfo_sha256"],
            })
        return receipt
    return {
        "schema_version": "oasis7.package_upgrade_operation_receipt.v1",
        "operation_id": request["operation_id"],
        "action_id": request["action_id"],
        "request_digest": request_digest,
        "phase": "staged",
        "outcome": "staged",
        "node_root": target["node_root"],
        "expected_instance_id": target["expected_instance_id"],
        "expected_generation": target["expected_generation"],
        "package_version": parameters["package_version"],
        "commit": parameters["commit"],
        "package_deb_sha256": inputs["package_deb_sha256"],
        "ops_tools_tar_sha256": inputs["ops_tools_tar_sha256"],
        "prepared_bundle_sha256": inputs["prepared_bundle_sha256"],
        "current_config_preimage_sha256": preimage_digest,
        "config_digest": request["preimage"]["config_digest"],
    }


def strict_json(path: Path) -> object:
    if path.is_symlink() or not path.is_file():
        fail(f"package operation metadata is not a regular file: {path}")
    metadata = path.stat()
    if metadata.st_uid != os.geteuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
        fail(f"package operation metadata is not owner-private: {path}")
    return json.loads(path.read_bytes())


def stop_proof(request: dict[str, object], lifecycle_data_dir: Path, stop_action_id: str) -> tuple[dict[str, object], str]:
    owner = request.get("lifecycle_owner")
    if not isinstance(owner, dict) or owner.get("data_dir") != str(lifecycle_data_dir.resolve(strict=True)):
        fail("package promotion lifecycle data directory differs from its staged owner")
    if stop_action_id != f"op:{request['operation_id']}:stop":
        fail("package promotion stop action ID differs from the original OperationRecord")
    action_path = lifecycle_data_dir / "actions" / (digest_bytes(json.dumps(stop_action_id, ensure_ascii=True, separators=(",", ":")).encode("utf-8")) + ".json")
    stop = strict_json(action_path)
    if not isinstance(stop, dict) or stop.get("api_version") != "oasis7.local-node-action/v1" or stop.get("kind") != "stop" or stop.get("action_id") != stop_action_id:
        fail("exact package stop action receipt is missing or invalid")
    target = stop.get("target")
    expected_payload = ["stop", owner["incarnation"], int(owner["generation"]), owner["config_digest"]]
    if not isinstance(target, dict) or (
        stop.get("expected_generation") != owner.get("generation")
        or target.get("incarnation") != owner.get("incarnation")
        or target.get("generation") != owner.get("generation")
        or target.get("config_digest") != owner.get("config_digest")
        or target.get("action_id") != owner.get("instance_action_id")
        or target.get("state") not in ("stopped", "failed")
        or target.get("runtime_pid") is not None
        or stop.get("payload_digest") != digest_bytes(json.dumps(expected_payload, ensure_ascii=True, separators=(",", ":")).encode("utf-8"))
    ):
        fail("package stop action is not bound to the exact staged owner")
    return stop, digest_bytes(json.dumps(stop, ensure_ascii=True, separators=(",", ":")).encode("utf-8"))


def require_stopped_instance(request: dict[str, object], lifecycle_data_dir: Path) -> None:
    owner = request.get("lifecycle_owner")
    instance = strict_json(lifecycle_data_dir / "instance.json")
    if not isinstance(owner, dict) or not isinstance(instance, dict) or (
        instance.get("api_version") != "oasis7.local-node-instance/v1"
        or instance.get("incarnation") != owner.get("incarnation")
        or instance.get("generation") != owner.get("generation")
        or instance.get("config_digest") != owner.get("config_digest")
        or instance.get("action_id") != owner.get("instance_action_id")
        or instance.get("state") not in ("stopped", "failed")
        or instance.get("runtime_pid") is not None
    ):
        fail("package promotion requires the exact stopped lifecycle instance")


def failed_start_proof(
    request: dict[str, object],
    lifecycle_data_dir: Path,
    promotion: dict[str, object],
    start_action_id: str,
) -> tuple[dict[str, object], str]:
    owner = request.get("lifecycle_owner")
    if not isinstance(owner, dict) or owner.get("data_dir") != str(lifecycle_data_dir.resolve(strict=True)):
        fail("package rollback lifecycle data directory differs from its staged owner")
    if start_action_id != f"op:{request['operation_id']}:start":
        fail("package rollback start action ID differs from the original OperationRecord")
    action_path = lifecycle_data_dir / "actions" / (digest_bytes(json.dumps(start_action_id, ensure_ascii=True, separators=(",", ":")).encode("utf-8")) + ".json")
    action = strict_json(action_path)
    target = action.get("target") if isinstance(action, dict) else None
    artifact = action.get("runtime_artifact") if isinstance(action, dict) else None
    if not isinstance(action, dict) or not isinstance(target, dict) or not isinstance(artifact, dict):
        fail("failed promoted start action receipt is missing or invalid")
    expected_generation = int(owner["generation"])
    expected_artifact = {
        "api_version": "oasis7.local-package-runtime-artifact/v1",
        "profile": request["profile"],
        "operation_id": request["operation_id"],
        "operation_request_digest": request["operation_record_digest"],
        "package_version": promotion["package_version"],
        "commit": promotion["commit"],
        "runtime_path": promotion["runtime_path"],
        "runtime_sha256": promotion["runtime_sha256"],
        "buildinfo_sha256": promotion["buildinfo_sha256"],
    }
    expected_payload = [
        "start_package",
        owner["config_digest"],
        expected_generation,
        expected_artifact,
    ]
    if (
        action.get("api_version") != "oasis7.local-node-action/v1"
        or action.get("action_id") != start_action_id
        or action.get("kind") != "start"
        or action.get("expected_generation") != expected_generation
        or target.get("api_version") != "oasis7.local-node-instance/v1"
        or target.get("action_id") != start_action_id
        or target.get("generation") != expected_generation + 1
        or target.get("config_digest") != owner.get("config_digest")
        or target.get("state") not in ("failed", "stopped")
        or target.get("runtime_pid") is not None
        or artifact != expected_artifact
        or action.get("payload_digest")
        != digest_bytes(json.dumps(expected_payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))
    ):
        fail("failed promoted start action is not bound to the exact package and OperationRecord")
    return action, digest_bytes(json.dumps(action, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))


def receipt_digest(value: object) -> str:
    return digest_bytes(json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8"))


def restored_runtime_identity(
    request: dict[str, object],
    node_root: Path,
    rollback_manifest: dict[str, object],
) -> dict[str, object]:
    current = rollback_manifest.get("current")
    if (
        not isinstance(current, dict)
        or current.get("kind") != "symlink"
        or not isinstance(current.get("target"), str)
        or not isinstance(current.get("resolved"), str)
    ):
        fail("package rollback requires a prior immutable release symlink")
    expected_platform = "local-native" if request.get("profile") == "local_native_package_v1" else "linux-x64"
    profile = request.get("profile")
    if profile not in ("public_testnet_linux_package_v1", "local_native_package_v1"):
        fail("package rollback requires a supported prior runtime profile")
    snapshot = read_named_release_snapshot(
        node_root,
        str(current["target"]),
        str(current["resolved"]),
        str(profile),
    )
    return {
        "restored_current_target": current["target"],
        "restored_current_resolved": current["resolved"],
        "package_version": snapshot["package_version"],
        "commit": snapshot["commit"],
        "platform": expected_platform,
        "runtime_path": snapshot["runtime_path"],
        "runtime_sha256": snapshot["runtime_sha256"],
        "buildinfo_sha256": snapshot["buildinfo_sha256"],
    }


def validate_prior_runtime_snapshot(request: dict[str, object], node_root: Path) -> dict[str, object] | None:
    if request.get("schema_version") != "oasis7.package_upgrade_operation_request.v2":
        return None
    prior_runtime = request.get("prior_runtime")
    preimage = request.get("preimage")
    current = preimage.get("current") if isinstance(preimage, dict) else None
    if not isinstance(prior_runtime, dict) or not isinstance(current, dict):
        fail("immutable package stage request has no prior runtime provenance")
    actual = restored_runtime_identity(request, node_root, {"current": current})
    if actual != prior_runtime:
        fail("prior release runtime/BUILDINFO provenance drifted from the immutable stage preimage")
    return actual


def verify_rollback_receipt(
    request: dict[str, object],
    node_root: Path,
    record_dir: Path,
    stage: dict[str, object],
    promotion: dict[str, object],
    promotion_intent: dict[str, object],
) -> dict[str, object]:
    rollback = strict_json(record_dir / "rollback.json")
    if not isinstance(rollback, dict):
        fail("persisted package rollback receipt is invalid")
    lifecycle_dir = Path(str(request["lifecycle_owner"]["data_dir"]))
    stop_action_id = str(promotion["stop_action_id"])
    _, stop_digest = stop_proof(request, lifecycle_dir, stop_action_id)
    start_action_id = f"op:{request['operation_id']}:start"
    _, start_digest = failed_start_proof(request, lifecycle_dir, promotion, start_action_id)
    rollback_transaction = Path(str(promotion["rollback_transaction"]))
    manifest = strict_json(rollback_transaction / "transaction.json")
    if not isinstance(manifest, dict) or manifest.get("phase") != "rolled_back":
        fail("package rollback receipt has no completed durable preimage restore")
    if live_preimage(
        node_root,
        allow_empty_config=request.get("profile") == "local_native_package_v1",
        profile=request.get("profile"),
    ) != request.get("preimage"):
        fail("restored package preimage differs from the original immutable request")
    identity = restored_runtime_identity(request, node_root, manifest)
    if identity != request.get("prior_runtime") or manifest.get("prior_runtime") != identity:
        fail("prior release runtime/BUILDINFO provenance differs from the immutable stage request")
    expected = {
        "schema_version": "oasis7.package_upgrade_rollback_receipt.v1",
        "profile": request["profile"],
        "operation_id": request["operation_id"],
        "action_id": f"pkg-{request['operation_id']}-rollback",
        "operation_request_digest": request["operation_record_digest"],
        "request_digest": digest_bytes(canonical(request)),
        "stage": stage,
        "promotion": promotion,
        "promotion_receipt_sha256": receipt_digest(promotion),
        "stop_action_id": stop_action_id,
        "stop_receipt_sha256": stop_digest,
        "failed_start_action_id": start_action_id,
        "failed_start_receipt_sha256": start_digest,
        "rollback_transaction": str(rollback_transaction),
        **identity,
    }
    candidate_plan = request.get("candidate_chain_plan")
    if candidate_plan is not None:
        expected["restored_node_config_digest"] = candidate_plan["prior_node_config_digest"]
        expected["restored_binding_digest"] = candidate_plan["prior_binding_digest"]
    if rollback != expected or promotion_intent.get("rollback_transaction") != str(rollback_transaction):
        fail("persisted package rollback receipt differs from the exact failed start and promotion")
    return rollback


def load_and_verify_record(record_dir: Path, node_root: Path, operation_id: str, action_id: str, expected_digest: str) -> tuple[dict[str, object], str]:
    if record_dir.is_symlink() or not record_dir.is_dir():
        fail(f"operation transaction is missing or not a directory: {record_dir}")
    check_private_directory(record_dir.parent)
    check_private_directory(record_dir)
    request_path = record_dir / "request.json"
    manifest_path = record_dir / "operation.json"
    receipt_path = record_dir / "receipt.json"
    for path in (request_path, manifest_path, receipt_path):
        if path.is_symlink() or not path.is_file():
            fail(f"operation transaction receipt is incomplete or corrupt: {path}")
        metadata = path.stat()
        if metadata.st_uid != os.geteuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
            fail(f"operation transaction metadata is not owner-private: {path}")
    request_bytes = request_path.read_bytes()
    try:
        request = json.loads(request_bytes)
    except Exception as exc:
        fail(f"operation request is corrupt: {exc}")
    if not isinstance(request, dict) or request_bytes != canonical(request) + b"\n":
        fail("operation request is not canonical")
    request_digest = digest_bytes(canonical(request))
    request_version = request.get("schema_version")
    if request_version == "oasis7.package_upgrade_operation_request.v2":
        if request.get("operation_record_digest") != expected_digest:
            fail("operation record digest does not match the supplied immutable digest")
        if request.get("profile") not in ("public_testnet_linux_package_v1", "local_native_package_v1"):
            fail("operation transaction profile is unsupported")
    elif request_version == "oasis7.package_upgrade_operation_request.v1":
        if request_digest != expected_digest:
            fail("operation request digest does not match the supplied immutable digest")
    else:
        fail("operation request schema is unsupported")
    if request.get("operation_id") != operation_id or request.get("action_id") != action_id:
        fail("operation/action ID does not match the persisted transaction")
    if request.get("action") != "package.stage":
        fail("persisted operation is not a stage transaction")
    target = request.get("target")
    if not isinstance(target, dict) or Path(str(target.get("node_root"))).resolve(strict=True) != node_root.resolve(strict=True):
        fail("operation transaction node-root binding does not match")
    validate_prior_runtime_snapshot(request, node_root)
    candidate_plan = request.get("candidate_chain_plan")
    if candidate_plan is not None:
        lifecycle_owner = request.get("lifecycle_owner")
        if not isinstance(lifecycle_owner, dict):
            fail("candidate-chain package request has no lifecycle owner")
        validate_candidate_chain_plan(
            candidate_plan,
            Path(str(lifecycle_owner["data_dir"])),
            check_preimage=False,
        )
        candidate_plan_json = request.get("candidate_chain_plan_json")
        try:
            parsed_plan_json = json.loads(str(candidate_plan_json))
        except Exception as exc:
            fail(f"immutable candidate-chain plan bytes are corrupt: {exc}")
        if (
            parsed_plan_json != candidate_plan
            or digest_bytes(str(candidate_plan_json).encode("utf-8"))
            != request.get("candidate_chain_plan_sha256")
            or candidate_plan.get("candidate_runtime_sha256") != request["inputs"].get("runtime_sha256")
            or candidate_plan.get("candidate_buildinfo_sha256") != request["inputs"].get("buildinfo_sha256")
        ):
            fail("immutable candidate-chain plan does not bind its runtime input or exact bytes")
        verify_candidate_chain_material_files(candidate_plan)

    try:
        manifest = json.loads(manifest_path.read_bytes())
    except Exception as exc:
        fail(f"operation manifest is corrupt: {exc}")
    expected_manifest = {
        "schema_version": (
            "oasis7.package_upgrade_operation.v2"
            if request_version == "oasis7.package_upgrade_operation_request.v2"
            else "oasis7.package_upgrade_operation.v1"
        ),
        "phase": "staged",
        "operation_id": operation_id,
        "action_id": action_id,
        "request_digest": request_digest,
        "artifacts": {
            "package.deb": request["inputs"]["package_deb_sha256"],
            "ops-tools.tar.gz": request["inputs"]["ops_tools_tar_sha256"],
            "ops-tools.manifest": request["inputs"]["ops_tools_manifest_sha256"],
            "prepared_bundle": request["inputs"]["prepared_bundle_sha256"],
        },
    }
    if manifest != expected_manifest:
        fail("operation manifest does not match the immutable request")

    artifacts = record_dir / "artifacts"
    prepared = record_dir / "prepared" / "oasis7-linux-x64"
    input_hashes = {
        "package.deb": digest_file(artifacts / "package.deb"),
        "ops-tools.tar.gz": digest_file(artifacts / "ops-tools.tar.gz"),
        "ops-tools.manifest": digest_file(artifacts / "ops-tools.manifest"),
    }
    if input_hashes != {
        "package.deb": request["inputs"]["package_deb_sha256"],
        "ops-tools.tar.gz": request["inputs"]["ops_tools_tar_sha256"],
        "ops-tools.manifest": request["inputs"]["ops_tools_manifest_sha256"],
    }:
        fail("persisted stage input artifact hash mismatch")
    if tree_digest(prepared) != request["inputs"]["prepared_bundle_sha256"]:
        fail("persisted prepared package hash mismatch")
    if digest_file(prepared / "BUILDINFO") != request["inputs"]["buildinfo_sha256"]:
        fail("persisted BUILDINFO hash mismatch")
    if digest_file(prepared / "bin" / "oasis7_chain_runtime") != request["inputs"]["runtime_sha256"]:
        fail("persisted runtime hash mismatch")
    receipt = expected_receipt(request, request_digest)
    expected_receipt_bytes = canonical(receipt) + b"\n"
    if receipt_path.read_bytes() != expected_receipt_bytes:
        fail("persisted operation receipt is corrupt")

    intent_path = record_dir / "promotion-intent.json"
    promotion_path = record_dir / "promotion.json"
    rollback_intent_path = record_dir / "rollback-intent.json"
    rollback_path = record_dir / "rollback.json"
    if candidate_plan is not None:
        if not intent_path.exists() and not intent_path.is_symlink():
            verify_candidate_lifecycle_state(candidate_plan, allow_partial=False, expected="old")
        elif rollback_path.exists() or rollback_path.is_symlink():
            verify_candidate_lifecycle_state(candidate_plan, allow_partial=False, expected="old")
        elif promotion_path.exists() and not (rollback_intent_path.exists() or rollback_intent_path.is_symlink()):
            verify_candidate_lifecycle_state(candidate_plan, allow_partial=False, expected="candidate")
        else:
            verify_candidate_lifecycle_state(candidate_plan, allow_partial=True)
    intent = None
    if intent_path.exists() or intent_path.is_symlink():
        intent = strict_json(intent_path)
        if not isinstance(intent, dict) or (
            intent.get("schema_version") != "oasis7.package_upgrade_promotion_intent.v1"
            or intent.get("operation_id") != operation_id
            or intent.get("operation_request_digest") != request.get("operation_record_digest")
            or intent.get("request_digest") != request_digest
            or intent.get("stage") != receipt
        ):
            fail("package promotion intent differs from the immutable stage receipt")
        _, stop_digest = stop_proof(request, Path(str(request["lifecycle_owner"]["data_dir"])), str(intent.get("stop_action_id", "")))
        if stop_digest != intent.get("stop_receipt_sha256"):
            fail("package promotion intent stop receipt digest changed")
        rollback_dir = Path(str(intent.get("rollback_transaction", "")))
        if rollback_dir != node_root / "package-upgrade-rollback" / f"operation-{operation_id}":
            fail("package promotion rollback transaction is not the deterministic operation path")
        rollback_manifest = strict_json(rollback_dir / "transaction.json")
        if (
            not isinstance(rollback_manifest, dict)
            or rollback_manifest.get("schema_version") != "oasis7.package_upgrade_rollback.v1"
            or rollback_manifest.get("node_root") != str(node_root)
            or rollback_manifest.get("operation_id") != operation_id
            or rollback_manifest.get("profile") != request.get("profile")
            or rollback_manifest.get("current") != request.get("preimage", {}).get("current")
            or rollback_manifest.get("prior_runtime") != request.get("prior_runtime")
        ):
            fail("package promotion rollback snapshot differs from the original operation")
        validate_lifecycle_snapshot(rollback_manifest, candidate_plan)
        current = node_root / "current"
        promoted_target = str(request["target"]["node_root"] + "/releases/" + str(request["parameters"]["package_version"]))
        current_is_original = live_preimage(
            node_root,
            allow_empty_config=request.get("profile") == "local_native_package_v1",
            profile=request.get("profile"),
        ).get("current") == request.get("preimage", {}).get("current")
        current_is_promoted = current.is_symlink() and os.readlink(current) in (promoted_target, str((node_root / "releases" / str(request["parameters"]["package_version"])).resolve(strict=False)))
        if not (current_is_original or current_is_promoted):
            fail("package current path is neither the staged preimage nor the exact promotion target")
        desired_files = intent.get("desired_files")
        if not isinstance(desired_files, dict):
            fail("package promotion intent has no desired metadata file digests")
        for entry in request["preimage"]["config_files"] + request["preimage"]["deployment_metadata"]:
            path = node_root / str(entry["path"])
            expected_old = entry.get("sha256")
            expected_new = desired_files.get(str(entry["path"]))
            try:
                actual = digest_file(path)
            except SystemExit:
                if entry.get("present") is False and expected_new is None and not path.exists() and not path.is_symlink():
                    continue
                raise
            if actual not in (expected_old, expected_new):
                fail(f"package promoted metadata drifted outside the staged transaction: {entry['path']}")
    else:
        actual_preimage = live_preimage(
            node_root,
            allow_empty_config=request.get("profile") == "local_native_package_v1",
            profile=request.get("profile"),
        )
        if actual_preimage != request.get("preimage"):
            fail("live current/config preimage drifted from the staged request")

    if promotion_path.exists() or promotion_path.is_symlink():
        promotion = strict_json(promotion_path)
        if not isinstance(promotion, dict) or promotion.get("schema_version") != "oasis7.package_upgrade_promotion_receipt.v1" or promotion.get("stage") != receipt:
            fail("persisted package promotion receipt is corrupt")
        if intent is None or any(promotion.get(key) != intent.get(key) for key in (
            "profile", "operation_id", "action_id", "operation_request_digest", "request_digest",
            "stop_action_id", "stop_receipt_sha256", "stopped_owner", "package_version", "commit",
            "prepared_bundle_sha256", "runtime_sha256", "runtime_path", "buildinfo_sha256",
            "promoted_current", "promoted_config_digest", "rollback_transaction",
            "candidate_chain_plan_sha256", "candidate_node_config_digest",
            "candidate_binding_digest", "candidate_anchor_digest",
        )):
            fail("persisted package promotion receipt differs from its durable intent")
        rollback_intent_path = record_dir / "rollback-intent.json"
        rollback_path = record_dir / "rollback.json"
        rollback_intent = None
        if rollback_intent_path.exists() or rollback_intent_path.is_symlink():
            rollback_intent = strict_json(rollback_intent_path)
            lifecycle_dir = Path(str(request["lifecycle_owner"]["data_dir"]))
            stop_action_id = str(promotion["stop_action_id"])
            _, stop_digest = stop_proof(request, lifecycle_dir, stop_action_id)
            failed_start_action_id = f"op:{operation_id}:start"
            _, failed_start_digest = failed_start_proof(
                request, lifecycle_dir, promotion, failed_start_action_id
            )
            expected_rollback_intent = {
                "schema_version": "oasis7.package_upgrade_rollback_intent.v1",
                "profile": request["profile"],
                "operation_id": operation_id,
                "action_id": f"pkg-{operation_id}-rollback",
                "operation_request_digest": request["operation_record_digest"],
                "request_digest": request_digest,
                "promotion_receipt_sha256": receipt_digest(promotion),
                "stop_action_id": stop_action_id,
                "stop_receipt_sha256": stop_digest,
                "failed_start_action_id": failed_start_action_id,
                "failed_start_receipt_sha256": failed_start_digest,
                "rollback_transaction": str(promotion["rollback_transaction"]),
            }
            if rollback_intent != expected_rollback_intent:
                fail("package rollback intent differs from the exact failed start and promotion")
            rollback_manifest = strict_json(
                Path(str(promotion["rollback_transaction"])) / "transaction.json"
            )
            if not isinstance(rollback_manifest, dict) or rollback_manifest.get("phase") not in (
                "completed",
                "rollback_started",
                "rolled_back",
            ):
                fail("package rollback intent has no valid promotion/restore phase")
        if rollback_path.exists() or rollback_path.is_symlink():
            if rollback_intent is None:
                fail("package rollback receipt has no durable rollback intent")
            return (
                verify_rollback_receipt(
                    request, node_root, record_dir, receipt, promotion, intent
                ),
                request_digest,
            )
        current = node_root / "current"
        promoted_target = Path(str(promotion["runtime_path"])).parent.parent
        if rollback_intent is not None:
            transaction = strict_json(Path(str(promotion["rollback_transaction"])) / "transaction.json")
            current_is_promoted = current.is_symlink() and current.resolve(strict=True) == promoted_target.resolve(strict=True)
            current_is_restored = live_preimage(
                node_root,
                allow_empty_config=request.get("profile") == "local_native_package_v1",
                profile=request.get("profile"),
            ) == request.get("preimage")
            if transaction.get("phase") == "completed" and not current_is_promoted:
                fail("rollback intent exists before the current release was fully promoted")
            if transaction.get("phase") in ("rollback_started", "rolled_back") and not (current_is_promoted or current_is_restored):
                fail("rollback recovery found a current release outside the exact preimage/promotion")
        elif not current.is_symlink() or current.resolve(strict=True) != promoted_target.resolve(strict=True):
            fail("persisted promotion receipt does not match the actual current release")
        runtime = Path(str(promotion["runtime_path"]))
        buildinfo = runtime.parent.parent / "BUILDINFO"
        if digest_file(runtime) != promotion["runtime_sha256"] or digest_file(buildinfo) != promotion["buildinfo_sha256"]:
            fail("persisted promotion receipt does not match the promoted runtime files")
        return promotion, request_digest
    return receipt, request_digest


def persist_stage(temp_dir: Path, record_dir: Path, request: dict[str, object], request_digest: str) -> None:
    if record_dir.exists() or record_dir.is_symlink():
        fail("operation transaction already exists; refusing to overwrite it")
    if digest_bytes(canonical(request)) != request_digest:
        fail("computed operation request digest changed before persistence")
    target = request["target"]
    root = Path(str(target["node_root"]))
    profile = request.get("profile")
    if isinstance(profile, str) and profile:
        current_binding = validate_operation_current(root, profile)
        if (
            current_binding.get("current") != request.get("preimage", {}).get("current")
            or current_binding.get("prior_runtime") != request.get("prior_runtime")
        ):
            fail("prior release runtime/BUILDINFO provenance drifted while staging")
    if live_preimage(
        root,
        allow_empty_config=request.get("profile") == "local_native_package_v1",
        profile=request.get("profile"),
    ) != request.get("preimage"):
        fail("live current/config preimage drifted while staging")
    candidate_plan = request.get("candidate_chain_plan")
    if candidate_plan is not None:
        lifecycle_owner = request.get("lifecycle_owner")
        if not isinstance(lifecycle_owner, dict):
            fail("candidate-chain stage request has no exact local lifecycle owner")
        validate_candidate_chain_plan(
            candidate_plan,
            Path(str(lifecycle_owner["data_dir"])),
            check_preimage=True,
        )
        if (
            request.get("candidate_chain_plan_sha256")
            != digest_bytes(str(request.get("candidate_chain_plan_json", "")).encode("utf-8"))
            or json.loads(str(request.get("candidate_chain_plan_json", ""))) != candidate_plan
            or candidate_plan.get("candidate_runtime_sha256") != request["inputs"].get("runtime_sha256")
            or candidate_plan.get("candidate_buildinfo_sha256") != request["inputs"].get("buildinfo_sha256")
        ):
            fail("candidate-chain plan digest/runtime/BUILDINFO differs from immutable package inputs")
    artifacts = temp_dir / "artifacts"
    prepared = temp_dir / "prepared" / "oasis7-linux-x64"
    if digest_file(artifacts / "package.deb") != request["inputs"]["package_deb_sha256"]:
        fail("staged package hash changed before persistence")
    if digest_file(artifacts / "ops-tools.tar.gz") != request["inputs"]["ops_tools_tar_sha256"]:
        fail("staged ops-tools hash changed before persistence")
    if tree_digest(prepared) != request["inputs"]["prepared_bundle_sha256"]:
        fail("prepared package tree changed before persistence")
    request_bytes = canonical(request) + b"\n"
    manifest = {
        "schema_version": (
            "oasis7.package_upgrade_operation.v2"
            if request["schema_version"] == "oasis7.package_upgrade_operation_request.v2"
            else "oasis7.package_upgrade_operation.v1"
        ),
        "phase": "staged",
        "operation_id": request["operation_id"],
        "action_id": request["action_id"],
        "request_digest": request_digest,
        "artifacts": {
            "package.deb": request["inputs"]["package_deb_sha256"],
            "ops-tools.tar.gz": request["inputs"]["ops_tools_tar_sha256"],
            "ops-tools.manifest": request["inputs"]["ops_tools_manifest_sha256"],
            "prepared_bundle": request["inputs"]["prepared_bundle_sha256"],
        },
    }
    receipt = expected_receipt(request, request_digest)
    write_immutable(temp_dir / "request.json", request_bytes)
    write_immutable(temp_dir / "operation.json", canonical(manifest) + b"\n")
    write_immutable(temp_dir / "receipt.json", canonical(receipt) + b"\n")
    fsync_tree(temp_dir)
    record_dir.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    check_private_directory(record_dir.parent)
    os.rename(temp_dir, record_dir)
    parent_fd = os.open(record_dir.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(parent_fd)
    finally:
        os.close(parent_fd)
    sys.stdout.buffer.write(canonical(receipt) + b"\n")


def atomic_write(path: Path, payload: bytes, metadata: dict[str, object] | None) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.package-promote-{os.getpid()}.tmp")
    try:
        with temporary.open("xb") as handle:
            handle.write(payload)
            if metadata is not None and metadata.get("present") is True:
                os.fchown(handle.fileno(), int(metadata["uid"]), int(metadata["gid"]))
                os.fchmod(handle.fileno(), int(metadata["mode"]))
            else:
                os.fchmod(handle.fileno(), 0o600)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        if temporary.exists() or temporary.is_symlink():
            temporary.unlink()


def promote_candidate_lifecycle_config(plan: dict[str, object]) -> None:
    verify_candidate_lifecycle_state(plan, allow_partial=True)
    for snapshot, payload, digest in (
        (
            plan["prior_node_binding"],
            str(plan["candidate_node_binding_bytes"]).encode("utf-8"),
            str(plan["candidate_node_binding_bytes_sha256"]),
        ),
        (
            plan["prior_node_config"],
            str(plan["candidate_node_config_bytes"]).encode("utf-8"),
            str(plan["candidate_node_config_bytes_sha256"]),
        ),
    ):
        path = Path(str(snapshot["path"]))
        if digest_file(path) == digest:
            continue
        atomic_write(path, payload, {
            "present": True,
            "uid": snapshot["uid"],
            "gid": snapshot["gid"],
            "mode": snapshot["mode"],
        })
    verify_candidate_lifecycle_state(plan, allow_partial=False, expected="candidate")


def promoted_metadata(request: dict[str, object], prepared: Path, node_root: Path) -> tuple[dict[str, bytes], str, str, int]:
    parameters = request["parameters"]
    inputs = request["inputs"]
    runtime = prepared / "bin" / "oasis7_chain_runtime"
    runtime_sha = digest_file(runtime)
    runtime_size = runtime.stat().st_size
    package_version = str(parameters["package_version"])
    commit = str(parameters["commit"])
    run_id = str(parameters["run_id"])
    artifact_ref = str(parameters["artifact_ref"])
    updated_by = f"p2p-public-testnet-package-node-upgrade {package_version} (run {run_id}, commit {commit})"
    changes: dict[str, bytes] = {}
    bundles = sorted((node_root / "config").rglob("public-testnet-governed-bootstrap-bundle-2026-06-06.json"))
    native_profile = request.get("profile") == "local_native_package_v1"
    candidate_plan = request.get("candidate_chain_plan")
    protected_candidate_paths = set()
    if candidate_plan is not None:
        protected_candidate_paths = {
            Path(str(item["path"])).resolve(strict=True)
            for item in candidate_plan["candidate_material"]
            if item.get("role") in ("manifest", "release_bundle")
        }
    if not bundles and not native_profile:
        fail(f"no governed bootstrap bundle found under {node_root / 'config'}")
    for path in bundles:
        if path.resolve(strict=True) in protected_candidate_paths:
            continue
        value = json.loads(path.read_bytes())
        runtime_build = value.setdefault("runtime_build", {})
        runtime_build.update({
            "git_commit": commit,
            "kind": "file",
            "path": str(node_root / "current" / "bin" / "oasis7_chain_runtime"),
            "resolved_path": str(node_root / "current" / "bin" / "oasis7_chain_runtime"),
            "ref": artifact_ref,
            "sha256": runtime_sha,
            "size_bytes": runtime_size,
            "package_version": package_version,
            "run_id": run_id,
            "updated_by": updated_by,
        })
        value["git_commit"] = commit
        value["updated_by"] = updated_by
        changes[str(path.relative_to(node_root))] = (json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8")
    changes["CURRENT_VERSION"] = (package_version + "\n").encode("utf-8")
    changes["DEPLOYED_BUILDINFO"] = ("\n".join([
        "workflow=Local Native Package" if native_profile else "workflow=Testnet Packages",
        f"run_id={run_id}",
        "repository=eng-cc/oasis7",
        f"commit={commit}",
        f"package_version={package_version}",
        "platform=local-native" if native_profile else "platform=linux-x64",
        f"runtime_sha256={runtime_sha}",
        f"runtime_size={runtime_size}",
        "",
    ])).encode("utf-8")
    desired_files = {name: digest_bytes(payload) for name, payload in changes.items()}
    return changes, runtime_sha, digest_file(prepared / "BUILDINFO"), runtime_size


def predicted_config_digest(request: dict[str, object], changes: dict[str, bytes]) -> str:
    old_entries = {
        str(entry["path"]): entry
        for entry in request["preimage"]["config_files"]
    }
    values = []
    for relative in sorted(old_entries):
        old = old_entries[relative]
        metadata = {
            "path": relative,
            "present": True,
            "sha256": digest_bytes(changes[relative]) if relative in changes else old["sha256"],
            "size": len(changes[relative]) if relative in changes else old["size"],
            "uid": old["uid"],
            "gid": old["gid"],
            "mode": old["mode"],
        }
        values.append(metadata)
    return digest_bytes(canonical(values))


def snapshot_operation(node_root: Path, transaction_dir: Path, request: dict[str, object]) -> None:
    profile = request.get("profile")
    selected_current: dict[str, object] | None = None
    if profile is not None:
        if profile not in ("public_testnet_linux_package_v1", "local_native_package_v1"):
            fail("package rollback snapshot has an unsupported deployment profile")
        selected_current = read_current_release_snapshot(node_root, str(profile))
        if (
            selected_current.get("current") != request.get("preimage", {}).get("current")
            or selected_current.get("prior_runtime") != request.get("prior_runtime")
        ):
            fail("package current or prior runtime differs from the original request preimage")
    transaction_parent = transaction_dir.parent
    transaction_parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    check_private_directory(transaction_parent)
    if transaction_dir.exists() or transaction_dir.is_symlink():
        manifest_path = transaction_dir / "transaction.json"
        manifest = strict_json(manifest_path)
        if (
            not isinstance(manifest, dict)
            or manifest.get("schema_version") != "oasis7.package_upgrade_rollback.v1"
            or manifest.get("node_root") != str(node_root)
            or manifest.get("operation_id") != request["operation_id"]
            or manifest.get("profile") != request.get("profile")
            or manifest.get("current") != request.get("preimage", {}).get("current")
            or manifest.get("prior_runtime") != request.get("prior_runtime")
            or manifest.get("candidate_chain_plan_sha256") != request.get("candidate_chain_plan_sha256")
        ):
            fail("existing package rollback transaction differs from the original operation")
        validate_lifecycle_snapshot(manifest, request.get("candidate_chain_plan"))
        return
    transaction_dir.mkdir(mode=0o700, parents=True)
    snapshot_dir = transaction_dir / "snapshot"
    snapshot_dir.mkdir(mode=0o700)
    current = node_root / "current"
    if selected_current is not None:
        current_state = selected_current["current"]
    elif current.is_symlink():
        current_state: dict[str, object] = {"kind": "symlink", "target": os.readlink(current), "resolved": str(current.resolve(strict=False))}
    elif current.is_dir():
        backup = snapshot_dir / "current-directory"
        shutil.copytree(current, backup, symlinks=True, copy_function=shutil.copy2)
        metadata = current.stat()
        current_state = {"kind": "directory", "backup": str(backup.relative_to(transaction_dir)), "uid": metadata.st_uid, "gid": metadata.st_gid, "mode": stat.S_IMODE(metadata.st_mode)}
    elif current.is_file():
        backup = snapshot_dir / "current-file"
        shutil.copy2(current, backup)
        metadata = current.stat()
        current_state = {"kind": "file", "backup": str(backup.relative_to(transaction_dir)), "sha256": digest_file(current), "uid": metadata.st_uid, "gid": metadata.st_gid, "mode": stat.S_IMODE(metadata.st_mode)}
    else:
        current_state = {"kind": "absent"}
    input_entries = request["preimage"]["config_files"] + request["preimage"]["deployment_metadata"]
    files = []
    for entry in input_entries:
        path = node_root / str(entry["path"])
        snapshot_entry = dict(entry)
        if entry.get("present") is True:
            backup = snapshot_dir / "files" / str(entry["path"])
            backup.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            shutil.copy2(path, backup)
            with backup.open("rb") as handle:
                os.fsync(handle.fileno())
            snapshot_entry["backup"] = str(backup.relative_to(transaction_dir))
        files.append(snapshot_entry)
    lifecycle_files = []
    candidate_plan = request.get("candidate_chain_plan")
    if candidate_plan is not None:
        validate_candidate_chain_plan(
            candidate_plan,
            Path(str(request["lifecycle_owner"]["data_dir"])),
            check_preimage=True,
        )
        for index, (snapshot, candidate_sha256) in enumerate((
            (candidate_plan["prior_node_config"], candidate_plan["candidate_node_config_bytes_sha256"]),
            (candidate_plan["prior_node_binding"], candidate_plan["candidate_node_binding_bytes_sha256"]),
        )):
            path = Path(str(snapshot["path"]))
            if path.is_symlink() or not path.is_file() or path.resolve(strict=True) != path:
                fail(f"candidate lifecycle preimage is not a regular canonical file: {path}")
            if path.read_bytes().decode("utf-8") != snapshot["bytes"]:
                fail(f"candidate lifecycle config changed before package snapshot: {path}")
            backup = snapshot_dir / "lifecycle" / f"{index}.json"
            backup.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            payload = snapshot["bytes"].encode("utf-8")
            with backup.open("xb") as handle:
                handle.write(payload)
                os.fchown(handle.fileno(), int(snapshot["uid"]), int(snapshot["gid"]))
                os.fchmod(handle.fileno(), int(snapshot["mode"]))
                handle.flush()
                os.fsync(handle.fileno())
            lifecycle_files.append({
                "path": str(path),
                "backup": str(backup.relative_to(transaction_dir)),
                "sha256": snapshot["sha256"],
                "candidate_sha256": candidate_sha256,
                "size": len(payload),
                "uid": snapshot["uid"],
                "gid": snapshot["gid"],
                "mode": snapshot["mode"],
            })
    if selected_current is not None:
        selected_after = read_current_release_snapshot(node_root, str(profile))
        if (
            selected_after.get("current") != request.get("preimage", {}).get("current")
            or selected_after.get("prior_runtime") != request.get("prior_runtime")
            or selected_after.get("current_link_identity")
            != selected_current.get("current_link_identity")
        ):
            shutil.rmtree(transaction_dir)
            fail("package current/link identity changed while capturing the rollback snapshot")
        current_state = selected_after["current"]
    manifest = {
        "schema_version": "oasis7.package_upgrade_rollback.v1",
        "phase": "snapshotted",
        "node_root": str(node_root),
        "profile": request.get("profile"),
        "current": current_state,
        "prior_runtime": request.get("prior_runtime"),
        "files": files,
        "lifecycle_files": lifecycle_files,
        "candidate_chain_plan_sha256": request.get("candidate_chain_plan_sha256"),
        "operation_id": request["operation_id"],
    }
    validate_lifecycle_snapshot(manifest, candidate_plan)
    write_immutable(transaction_dir / "transaction.json", (json.dumps(manifest, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    fsync_tree(transaction_dir)


def promote_record(args: list[str], *, emit: bool = True) -> None:
    if len(args) != 13:
        fail("invalid package promotion arguments")
    (root_text, operation_id, stage_action_id, operation_record_digest, request_digest,
     lifecycle_dir_text, expected_incarnation, expected_generation, expected_config_digest,
     expected_instance_action_id, promotion_action_id, stop_action_id, _profile) = args
    node_root = Path(root_text).resolve(strict=True)
    record_dir = node_root / "package-upgrade-operations" / operation_id
    persisted, stored_request_digest = load_and_verify_record(record_dir, node_root, operation_id, stage_action_id, operation_record_digest)
    if persisted.get("schema_version") == "oasis7.package_upgrade_operation_receipt.v2":
        stage = persisted
    elif persisted.get("schema_version") == "oasis7.package_upgrade_promotion_receipt.v1":
        stage = persisted.get("stage")
    else:
        stage = None
    if not isinstance(stage, dict) or stored_request_digest != request_digest:
        fail("package promotion stage receipt differs from the exact operation request")
    request = json.loads((record_dir / "request.json").read_bytes())
    validate_prior_runtime_snapshot(request, node_root)
    if request.get("schema_version") == "oasis7.package_upgrade_operation_request.v2":
        revalidate_runtime_compatibility(request, "candidate")
    owner = request.get("lifecycle_owner")
    if request.get("profile") != _profile or not isinstance(owner, dict) or (
        owner.get("incarnation") != expected_incarnation
        or owner.get("generation") != int(expected_generation)
        or owner.get("config_digest") != expected_config_digest
        or owner.get("instance_action_id") != expected_instance_action_id
    ):
        fail("package promotion lifecycle owner differs from the immutable stage request")
    lifecycle_dir = Path(lifecycle_dir_text).resolve(strict=True)
    stop, stop_digest = stop_proof(request, lifecycle_dir, stop_action_id)
    if promotion_action_id != f"pkg-{operation_id}-promote":
        fail("package promotion action ID is not the deterministic OperationRecord action")
    promotion_path = record_dir / "promotion.json"
    if promotion_path.exists() or promotion_path.is_symlink():
        returned = strict_json(promotion_path)
        intent_path = record_dir / "promotion-intent.json"
        intent = strict_json(intent_path)
        if not isinstance(returned, dict) or not isinstance(intent, dict):
            fail("persisted package promotion receipt is invalid")
        if (
            returned.get("schema_version") != "oasis7.package_upgrade_promotion_receipt.v1"
            or returned.get("stage") != stage
            or returned.get("action_id") != promotion_action_id
            or returned.get("stop_action_id") != stop_action_id
            or intent.get("stop_action_id") != stop_action_id
            or returned.get("stop_receipt_sha256") != stop_digest
            or returned.get("stopped_owner") != stop.get("target")
        ):
            fail("package promotion retry differs from its exact stopped owner")
        if emit:
            sys.stdout.buffer.write(canonical(returned) + b"\n")
        return

    require_stopped_instance(request, lifecycle_dir)

    prepared = record_dir / "prepared" / "oasis7-linux-x64"
    if tree_digest(prepared) != request["inputs"]["prepared_bundle_sha256"]:
        fail("staged package tree changed before promotion")
    changes, runtime_sha, buildinfo_sha, runtime_size = promoted_metadata(request, prepared, node_root)
    desired_files = {name: digest_bytes(payload) for name, payload in changes.items()}
    transaction_dir = node_root / "package-upgrade-rollback" / f"operation-{operation_id}"
    intent_path = record_dir / "promotion-intent.json"
    candidate_plan = request.get("candidate_chain_plan")
    if candidate_plan is not None:
        lifecycle_dir = Path(str(owner["data_dir"]))
        validate_candidate_chain_plan(candidate_plan, lifecycle_dir, check_preimage=not (intent_path.exists() or intent_path.is_symlink()))
        verify_candidate_chain_material_files(candidate_plan)
        verify_candidate_lifecycle_state(
            candidate_plan,
            allow_partial=intent_path.exists() or intent_path.is_symlink(),
            expected=None if intent_path.exists() or intent_path.is_symlink() else "old",
        )
    if intent_path.exists() or intent_path.is_symlink():
        intent = strict_json(intent_path)
        if not isinstance(intent, dict) or (
            intent.get("schema_version") != "oasis7.package_upgrade_promotion_intent.v1"
            or intent.get("request_digest") != request_digest
            or intent.get("operation_request_digest") != operation_record_digest
            or intent.get("stop_action_id") != stop_action_id
            or intent.get("stop_receipt_sha256") != stop_digest
            or intent.get("stopped_owner") != stop["target"]
            or intent.get("desired_files") != desired_files
            or intent.get("runtime_sha256") != runtime_sha
            or intent.get("candidate_chain_plan_sha256") != request.get("candidate_chain_plan_sha256")
            or intent.get("candidate_node_config_digest") != (candidate_plan or {}).get("candidate_node_config_digest")
            or intent.get("candidate_binding_digest") != (candidate_plan or {}).get("candidate_binding_digest")
            or intent.get("candidate_anchor_digest") != (candidate_plan or {}).get("candidate_anchor_digest")
        ):
            fail("package promotion retry differs from its durable intent")
    else:
        if live_preimage(
            node_root,
            allow_empty_config=request.get("profile") == "local_native_package_v1",
            profile=request.get("profile"),
        ) != request.get("preimage"):
            fail("live package preimage drifted before exact stopped-owner promotion")
        validate_prior_runtime_snapshot(request, node_root)
        snapshot_operation(node_root, transaction_dir, request)
        intent = {
            "schema_version": "oasis7.package_upgrade_promotion_intent.v1",
            "profile": _profile,
            "operation_id": operation_id,
            "action_id": promotion_action_id,
            "operation_request_digest": operation_record_digest,
            "request_digest": request_digest,
            "stage": stage,
            "stop_action_id": stop_action_id,
            "stop_receipt_sha256": stop_digest,
            "stopped_owner": stop["target"],
            "package_version": request["parameters"]["package_version"],
            "commit": request["parameters"]["commit"],
            "prepared_bundle_sha256": request["inputs"]["prepared_bundle_sha256"],
            "runtime_sha256": runtime_sha,
            "runtime_path": str(node_root / "releases" / str(request["parameters"]["package_version"]) / "bin" / "oasis7_chain_runtime"),
            "buildinfo_sha256": buildinfo_sha,
            "promoted_current": str(node_root / "current"),
            "promoted_config_digest": predicted_config_digest(request, changes),
            "rollback_transaction": str(transaction_dir),
            "desired_files": desired_files,
            "candidate_chain_plan_sha256": request.get("candidate_chain_plan_sha256"),
            "candidate_node_config_digest": (candidate_plan or {}).get("candidate_node_config_digest"),
            "candidate_binding_digest": (candidate_plan or {}).get("candidate_binding_digest"),
            "candidate_anchor_digest": (candidate_plan or {}).get("candidate_anchor_digest"),
        }
        write_immutable(intent_path, canonical(intent) + b"\n")
        fsync_tree(record_dir)

    release_dir = node_root / "releases" / str(request["parameters"]["package_version"])
    temporary_release = node_root / "releases" / f".{request['parameters']['package_version']}.{operation_id}.tmp"
    release_dir.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
    if release_dir.is_symlink():
        fail("package release path is a symlink")
    if not release_dir.exists():
        if temporary_release.exists() or temporary_release.is_symlink():
            if temporary_release.is_symlink() or not temporary_release.is_dir() or tree_digest(temporary_release) != request["inputs"]["prepared_bundle_sha256"]:
                fail("incomplete package release path does not match the staged immutable bundle")
        else:
            shutil.copytree(prepared, temporary_release, symlinks=False, copy_function=shutil.copy2)
            fsync_tree(temporary_release)
        if tree_digest(temporary_release) != request["inputs"]["prepared_bundle_sha256"]:
            fail("promoted release copy differs from the staged immutable bundle")
        os.rename(temporary_release, release_dir)
        directory_fd = os.open(release_dir.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    elif tree_digest(release_dir) != request["inputs"]["prepared_bundle_sha256"]:
        fail("existing package release conflicts with the staged immutable bundle")

    journal_transaction_phase(transaction_dir, "release_promoted")

    original_metadata = {
        str(entry["path"]): entry
        for entry in request["preimage"]["config_files"] + request["preimage"]["deployment_metadata"]
    }
    validate_prior_runtime_snapshot(request, node_root)
    for relative, payload in changes.items():
        path = node_root / relative
        entry = original_metadata.get(relative)
        if entry is None:
            fail("package promotion metadata path is outside the staged preimage")
        if entry.get("present") is True and path.exists() and digest_file(path) not in (entry.get("sha256"), desired_files[relative]):
            fail(f"package promotion metadata drifted before recovery: {relative}")
        atomic_write(path, payload, entry)

    journal_transaction_phase(transaction_dir, "metadata_promoted")

    if candidate_plan is not None:
        promote_candidate_lifecycle_config(candidate_plan)
    journal_transaction_phase(transaction_dir, "lifecycle_config_promoted")

    current = node_root / "current"
    target = str(release_dir)
    validate_prior_runtime_snapshot(request, node_root)
    resolved_current = current.resolve(strict=False)
    if current.is_symlink() and resolved_current == release_dir.resolve(strict=True):
        pass
    else:
        if live_preimage(
            node_root,
            allow_empty_config=request.get("profile") == "local_native_package_v1",
            profile=request.get("profile"),
        ).get("current") != request["preimage"].get("current"):
            fail("current release changed outside the exact package promotion")
        temporary_link = current.with_name(f".{current.name}.operation-promote-{operation_id}.tmp")
        if temporary_link.exists() or temporary_link.is_symlink():
            fail("package promotion temporary current path already exists")
        if current.is_dir() and not current.is_symlink():
            backup = transaction_dir / "current-directory-live"
            if backup.exists() or backup.is_symlink():
                fail("package promotion current-directory backup already exists")
            os.replace(current, backup)
        os.symlink(target, temporary_link)
        directory_fd = os.open(current.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        os.replace(temporary_link, current)
        directory_fd = os.open(current.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)

    journal_transaction_phase(transaction_dir, "current_promoted", target)

    config_files = [file_state(node_root / str(entry["path"]), node_root) for entry in request["preimage"]["config_files"]]
    promoted_config_digest = digest_bytes(canonical(config_files))
    if promoted_config_digest != intent["promoted_config_digest"]:
        fail("promoted governed configuration digest differs from the immutable promotion intent")
    promotion = {key: value for key, value in intent.items() if key != "desired_files"}
    promotion["schema_version"] = "oasis7.package_upgrade_promotion_receipt.v1"
    write_immutable(promotion_path, canonical(promotion) + b"\n")
    journal_transaction_phase(transaction_dir, "completed", target)
    fsync_tree(record_dir)
    if emit:
        sys.stdout.buffer.write(canonical(promotion) + b"\n")


def rollback_record(args: list[str], *, emit: bool = True) -> None:
    if len(args) not in (16, 18):
        fail("invalid package rollback arguments")
    (root_text, operation_id, stage_action_id, operation_record_digest, request_digest,
     lifecycle_dir_text, expected_incarnation, expected_generation, expected_config_digest,
     expected_instance_action_id, promotion_action_id, stop_action_id,
     failed_start_action_id, failed_start_receipt_digest, rollback_action_id, _profile) = args[:16]
    compatibility_path, compatibility_sha256 = args[16:] if len(args) == 18 else ("", "")
    node_root = Path(root_text).resolve(strict=True)
    record_dir = node_root / "package-upgrade-operations" / operation_id
    persisted, stored_request_digest = load_and_verify_record(
        record_dir, node_root, operation_id, stage_action_id, operation_record_digest
    )
    if stored_request_digest != request_digest:
        fail("package rollback stage receipt differs from the exact operation request")
    if persisted.get("schema_version") == "oasis7.package_upgrade_rollback_receipt.v1":
        if persisted.get("action_id") != rollback_action_id or persisted.get("failed_start_receipt_sha256") != failed_start_receipt_digest:
            fail("package rollback retry differs from its durable receipt")
        if emit:
            sys.stdout.buffer.write(canonical(persisted) + b"\n")
        return
    if persisted.get("schema_version") == "oasis7.package_upgrade_operation_receipt.v2":
        fail("package rollback requires an exact promoted package receipt")
    if persisted.get("schema_version") != "oasis7.package_upgrade_promotion_receipt.v1":
        fail("package rollback has no exact promotion receipt")
    promotion = persisted
    stage = promotion.get("stage")
    request = json.loads((record_dir / "request.json").read_bytes())
    validate_prior_runtime_snapshot(request, node_root)
    if request.get("schema_version") == "oasis7.package_upgrade_operation_request.v2":
        if (
            compatibility_path != request.get("runtime_compatibility_plan_path")
            or compatibility_sha256 != request.get("runtime_compatibility_plan_sha256")
        ):
            fail("rollback compatibility plan differs from the exact immutable stage request")
        revalidate_runtime_compatibility(request, "prior")
    owner = request.get("lifecycle_owner")
    if request.get("profile") != _profile or not isinstance(owner, dict) or (
        owner.get("incarnation") != expected_incarnation
        or owner.get("generation") != int(expected_generation)
        or owner.get("config_digest") != expected_config_digest
        or owner.get("instance_action_id") != expected_instance_action_id
    ):
        fail("package rollback owner differs from the immutable stage request")
    lifecycle_dir = Path(lifecycle_dir_text).resolve(strict=True)
    if lifecycle_dir != Path(str(owner.get("data_dir"))).resolve(strict=True):
        fail("package rollback lifecycle data directory differs from the staged owner")
    if promotion_action_id != f"pkg-{operation_id}-promote" or rollback_action_id != f"pkg-{operation_id}-rollback":
        fail("package rollback action IDs are not deterministic OperationRecord actions")
    if stop_action_id != promotion.get("stop_action_id") or promotion.get("stage") != stage:
        fail("package rollback does not name the exact promoted stop receipt")
    _, stop_digest = stop_proof(request, lifecycle_dir, stop_action_id)
    if stop_digest != promotion.get("stop_receipt_sha256"):
        fail("package rollback stop receipt differs from the exact promotion")
    start_action, start_digest = failed_start_proof(
        request, lifecycle_dir, promotion, failed_start_action_id
    )
    if start_digest != failed_start_receipt_digest:
        fail("package rollback failed-start receipt digest differs from the exact action")
    current_instance = strict_json(lifecycle_dir / "instance.json")
    if current_instance != start_action.get("target"):
        fail("package rollback requires the exact failed promoted-start instance to remain current")

    transaction_dir = Path(str(promotion["rollback_transaction"]))
    expected_transaction_dir = node_root / "package-upgrade-rollback" / f"operation-{operation_id}"
    check_private_directory(transaction_dir)
    transaction = strict_json(transaction_dir / "transaction.json")
    if (
        not isinstance(transaction, dict)
        or transaction.get("operation_id") != operation_id
        or transaction.get("node_root") != str(node_root)
        or transaction.get("profile") != request.get("profile")
        or transaction.get("current") != request.get("preimage", {}).get("current")
        or transaction.get("prior_runtime") != request.get("prior_runtime")
        or transaction.get("candidate_chain_plan_sha256") != request.get("candidate_chain_plan_sha256")
        or transaction_dir.resolve(strict=True) != expected_transaction_dir.resolve(strict=False)
    ):
        fail("package rollback snapshot differs from the exact operation")
    validate_lifecycle_snapshot(transaction, request.get("candidate_chain_plan"))
    expected_preimage_files = request["preimage"]["config_files"] + request["preimage"]["deployment_metadata"]
    snapshot_files = transaction.get("files")
    if not isinstance(snapshot_files, list) or len(snapshot_files) != len(expected_preimage_files):
        fail("package rollback snapshot file set differs from the exact operation preimage")
    expected_by_path = {entry["path"]: entry for entry in expected_preimage_files}
    for snapshot in snapshot_files:
        if not isinstance(snapshot, dict) or snapshot.get("path") not in expected_by_path:
            fail("package rollback snapshot contains a file outside the exact operation preimage")
        expected_file = expected_by_path.pop(snapshot["path"])
        if any(snapshot.get(key) != value for key, value in expected_file.items()):
            fail("package rollback snapshot metadata differs from the exact operation preimage")
    if expected_by_path:
        fail("package rollback snapshot omits a file from the exact operation preimage")
    preimage_current = request.get("preimage", {}).get("current")
    if (
        not isinstance(preimage_current, dict)
        or preimage_current.get("kind") != "symlink"
        or transaction.get("current") != preimage_current
    ):
        fail("package rollback current snapshot differs from the exact operation preimage")
    if transaction.get("phase") not in ("completed", "current_promoted", "rollback_started", "rolled_back"):
        fail("package rollback requires a fully promoted release")
    identity = restored_runtime_identity(request, node_root, transaction)
    if identity != request.get("prior_runtime"):
        fail("prior release runtime/BUILDINFO provenance drifted from the immutable stage request")
    rollback_intent_path = record_dir / "rollback-intent.json"
    rollback_intent = {
        "schema_version": "oasis7.package_upgrade_rollback_intent.v1",
        "profile": request["profile"],
        "operation_id": operation_id,
        "action_id": rollback_action_id,
        "operation_request_digest": operation_record_digest,
        "request_digest": request_digest,
        "promotion_receipt_sha256": receipt_digest(promotion),
        "stop_action_id": stop_action_id,
        "stop_receipt_sha256": stop_digest,
        "failed_start_action_id": failed_start_action_id,
        "failed_start_receipt_sha256": start_digest,
        "rollback_transaction": str(transaction_dir),
    }
    if rollback_intent_path.exists() or rollback_intent_path.is_symlink():
        if strict_json(rollback_intent_path) != rollback_intent:
            fail("package rollback retry differs from its immutable intent")
    else:
        if transaction.get("phase") not in ("completed", "current_promoted"):
            fail("package rollback has no durable intent for its interrupted restore")
        write_immutable(rollback_intent_path, canonical(rollback_intent) + b"\n")
        fsync_tree(record_dir)

    validate_prior_runtime_snapshot(request, node_root)
    transaction_before_restore = strict_json(transaction_dir / "transaction.json")
    if (
        not isinstance(transaction_before_restore, dict)
        or transaction_before_restore.get("profile") != request.get("profile")
        or transaction_before_restore.get("current") != request.get("preimage", {}).get("current")
        or transaction_before_restore.get("prior_runtime") != request.get("prior_runtime")
    ):
        fail("package rollback snapshot provenance differs from the immutable stage request")
    restore_operation_snapshot(transaction_dir)
    if live_preimage(
        node_root,
        allow_empty_config=request.get("profile") == "local_native_package_v1",
        profile=request.get("profile"),
    ) != request.get("preimage"):
        fail("package rollback did not restore the original current/config preimage")
    restored = restored_runtime_identity(request, node_root, strict_json(transaction_dir / "transaction.json"))
    if restored != identity:
        fail("package rollback restored a different prior runtime identity")
    rollback = {
        "schema_version": "oasis7.package_upgrade_rollback_receipt.v1",
        "profile": request["profile"],
        "operation_id": operation_id,
        "action_id": rollback_action_id,
        "operation_request_digest": operation_record_digest,
        "request_digest": request_digest,
        "stage": stage,
        "promotion": promotion,
        "promotion_receipt_sha256": receipt_digest(promotion),
        "stop_action_id": stop_action_id,
        "stop_receipt_sha256": stop_digest,
        "failed_start_action_id": failed_start_action_id,
        "failed_start_receipt_sha256": start_digest,
        "rollback_transaction": str(transaction_dir),
        **restored,
    }
    candidate_plan = request.get("candidate_chain_plan")
    if candidate_plan is not None:
        rollback["restored_node_config_digest"] = candidate_plan["prior_node_config_digest"]
        rollback["restored_binding_digest"] = candidate_plan["prior_binding_digest"]
    write_immutable(record_dir / "rollback.json", canonical(rollback) + b"\n")
    fsync_tree(record_dir)
    if emit:
        sys.stdout.buffer.write(canonical(rollback) + b"\n")


def query_record(node_root: Path, operation_id: str, action_id: str, expected_digest: str) -> dict[str, object]:
    record_dir = node_root / "package-upgrade-operations" / operation_id
    if not record_dir.exists() and not record_dir.is_symlink():
        return {"schema_version": "oasis7.package_upgrade_operation_absent.v1"}
    receipt, _ = load_and_verify_record(record_dir, node_root, operation_id, action_id, expected_digest)
    if receipt.get("schema_version") != "oasis7.package_upgrade_operation_receipt.v2":
        return receipt
    request = strict_json(record_dir / "request.json")
    if not isinstance(request, dict):
        fail("package stage request is invalid during reconciliation")
    helper_request_digest = digest_bytes(canonical(request))
    lifecycle_owner = request.get("lifecycle_owner")
    if not isinstance(lifecycle_owner, dict):
        fail("package reconciliation request has no exact lifecycle owner")
    intent_path = record_dir / "promotion-intent.json"
    promotion_path = record_dir / "promotion.json"
    rollback_intent_path = record_dir / "rollback-intent.json"
    rollback_path = record_dir / "rollback.json"
    if rollback_intent_path.exists() or rollback_intent_path.is_symlink():
        if rollback_path.exists() or rollback_path.is_symlink():
            return strict_json(rollback_path)
        if not (promotion_path.exists() or promotion_path.is_symlink()):
            fail("rollback intent exists without the exact promoted receipt")
        rollback_intent = strict_json(rollback_intent_path)
        if not isinstance(rollback_intent, dict):
            fail("package rollback intent is invalid during reconciliation")
        promotion = strict_json(promotion_path)
        rollback_args = [
            str(node_root), operation_id, action_id,
            str(request.get("operation_record_digest", "")), helper_request_digest,
            str(lifecycle_owner["data_dir"]), str(lifecycle_owner["incarnation"]),
            str(lifecycle_owner["generation"]), str(lifecycle_owner["config_digest"]),
            str(lifecycle_owner["instance_action_id"]), str(promotion["action_id"]),
            str(rollback_intent["stop_action_id"]),
            str(rollback_intent["failed_start_action_id"]),
            str(rollback_intent["failed_start_receipt_sha256"]),
            str(rollback_intent["action_id"]), str(request["profile"]),
            str(request.get("runtime_compatibility_plan_path", "")),
            str(request.get("runtime_compatibility_plan_sha256", "")),
        ]
        rollback_record(rollback_args, emit=False)
        return strict_json(rollback_path)
    if intent_path.exists() or intent_path.is_symlink():
        if promotion_path.exists() or promotion_path.is_symlink():
            return strict_json(promotion_path)
        intent = strict_json(intent_path)
        if not isinstance(intent, dict):
            fail("package promotion intent is invalid during reconciliation")
        promote_args = [
            str(node_root), operation_id, action_id,
            str(request.get("operation_record_digest", "")), helper_request_digest,
            str(lifecycle_owner["data_dir"]), str(lifecycle_owner["incarnation"]),
            str(lifecycle_owner["generation"]), str(lifecycle_owner["config_digest"]),
            str(lifecycle_owner["instance_action_id"]), str(intent["action_id"]),
            str(intent["stop_action_id"]), str(request["profile"]),
        ]
        promote_record(promote_args, emit=False)
        return strict_json(promotion_path)
    return receipt


def main() -> None:
    command = sys.argv[1]
    args = sys.argv[2:]
    if command == "validate-current":
        if len(args) != 2:
            fail("invalid package current validation arguments")
        root = Path(args[0]).resolve(strict=True)
        result = validate_operation_current(root, args[1])
        sys.stdout.buffer.write(canonical(result) + b"\n")
        return
    if command == "request":
        request = request_for(args)
        print(canonical(request).decode("utf-8"))
        return
    if command == "persist":
        temp_dir, record_dir, request_json, request_digest = args
        request = json.loads(request_json)
        persist_stage(Path(temp_dir), Path(record_dir), request, request_digest)
        return
    if command == "query":
        node_root, operation_id, action_id, expected_digest = args
        receipt = query_record(Path(node_root), operation_id, action_id, expected_digest)
        sys.stdout.buffer.write(canonical(receipt) + b"\n")
        return
    if command == "promote":
        promote_record(args)
        return
    if command == "rollback":
        rollback_record(args)
        return
    if command == "retry":
        if len(args) not in (12, 18):
            fail("invalid package stage retry arguments")
        (node_root, operation_id, action_id, expected_digest, package_source,
         ops_source, expected_instance, expected_generation, package_version,
         commit, run_id, artifact_ref) = args[:12]
        profile, operation_record_digest, expected_incarnation, expected_config_digest, expected_instance_action_id, lifecycle_data_dir = (
            args[12:] if len(args) == 18 else ("", "", "", "", "", "")
        )
        record_dir = Path(node_root) / "package-upgrade-operations" / operation_id
        request = json.loads((record_dir / "request.json").read_bytes())
        stored_digest = digest_bytes(canonical(request))
        supplied_digest = expected_digest or (
            request.get("operation_record_digest", stored_digest)
        )
        receipt, request_digest = load_and_verify_record(record_dir, Path(node_root), operation_id, action_id, supplied_digest)
        expected_parameters = {"package_version": package_version, "commit": commit, "run_id": run_id, "artifact_ref": artifact_ref}
        target = request["target"]
        if request["parameters"] != expected_parameters or target["expected_instance_id"] != expected_instance or target["expected_generation"] != int(expected_generation):
            fail("operation ID was reused with a different immutable request preimage")
        if digest_file(Path(package_source)) != request["inputs"]["package_deb_sha256"] or digest_file(Path(ops_source)) != request["inputs"]["ops_tools_tar_sha256"]:
            fail("operation ID was reused with different package or ops-tools inputs")
        if profile:
            owner = request.get("lifecycle_owner")
            if request.get("profile") != profile or request.get("operation_record_digest") != operation_record_digest or not isinstance(owner, dict):
                fail("operation ID was reused with a different immutable OperationRecord binding")
            if owner != {
                "data_dir": str(Path(lifecycle_data_dir).resolve(strict=True)),
                "incarnation": expected_incarnation,
                "generation": int(expected_generation),
                "config_digest": expected_config_digest,
                "instance_action_id": expected_instance_action_id,
            }:
                fail("operation ID was reused with a different exact lifecycle owner")
        sys.stdout.buffer.write(canonical(receipt) + b"\n")
        return
    if command == "prepare-parent":
        parent, temp_dir = map(Path, args)
        parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        check_private_directory(parent)
        parent_fd = os.open(parent.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
        if temp_dir.exists() or temp_dir.is_symlink():
            if temp_dir.is_symlink() or not temp_dir.is_dir():
                fail(f"incomplete operation path is unsafe: {temp_dir}")
            metadata = temp_dir.lstat()
            if metadata.st_uid != os.geteuid():
                fail(f"incomplete operation path is not owned by this user: {temp_dir}")
            shutil.rmtree(temp_dir)
        return
    if command == "acquire-lock":
        root, operation_id, pid_text = args
        root_path = Path(root).resolve(strict=True)
        lock_path = root_path / ".package-upgrade.lock"
        temporary_lock = root_path / ".package-upgrade.lock.new"
        recovery_path = root_path / ".package-upgrade-lock-recovery"
        if recovery_path.is_symlink():
            fail(f"upgrade lock recovery path is unsafe: {recovery_path}")
        recovery_fd = os.open(recovery_path, os.O_CREAT | os.O_RDWR | getattr(os, "O_NOFOLLOW", 0), 0o600)
        try:
            os.fchmod(recovery_fd, 0o600)
            fcntl.flock(recovery_fd, fcntl.LOCK_EX)
            if temporary_lock.exists() or temporary_lock.is_symlink():
                if temporary_lock.is_symlink() or not temporary_lock.is_dir():
                    fail(f"incomplete upgrade lock path is unsafe: {temporary_lock}")
                if temporary_lock.stat().st_uid != os.geteuid():
                    fail(f"incomplete upgrade lock path is not owned by this user: {temporary_lock}")
                shutil.rmtree(temporary_lock)
            if lock_path.exists() or lock_path.is_symlink():
                if lock_path.is_symlink() or not lock_path.is_dir():
                    fail(f"upgrade lock path is unsafe: {lock_path}")
                owner_path = lock_path / "owner.json"
                if owner_path.is_symlink() or not owner_path.is_file():
                    fail("an upgrade lock exists without recoverable operation ownership; refusing to guess")
                try:
                    owner = json.loads(owner_path.read_bytes())
                    owner_pid = int(owner["pid"])
                    if (
                        not isinstance(owner, dict)
                        or owner.get("schema_version") != "oasis7.package_upgrade_lock_owner.v1"
                        or not isinstance(owner.get("operation_id"), str)
                        or owner_pid <= 1
                    ):
                        fail("upgrade lock owner is invalid; refusing to guess")
                except Exception as exc:
                    fail(f"upgrade lock owner is corrupt; refusing to guess: {exc}")
                try:
                    os.kill(owner_pid, 0)
                except ProcessLookupError:
                    shutil.rmtree(lock_path)
                except PermissionError:
                    fail(f"another package upgrade is active under pid {owner_pid}")
                else:
                    fail(f"another package upgrade is active under pid {owner_pid}")
            temporary_lock.mkdir(mode=0o700)
            owner = {"schema_version": "oasis7.package_upgrade_lock_owner.v1", "pid": int(pid_text), "operation_id": operation_id}
            write_immutable(temporary_lock / "owner.json", canonical(owner) + b"\n")
            directory_fd = os.open(temporary_lock, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
            os.rename(temporary_lock, lock_path)
            root_fd = os.open(root_path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
            try:
                os.fsync(root_fd)
            finally:
                os.close(root_fd)
        finally:
            fcntl.flock(recovery_fd, fcntl.LOCK_UN)
            os.close(recovery_fd)
        return
    if command == "release-lock":
        root, operation_id, pid_text = args
        root_path = Path(root).resolve(strict=True)
        lock_path = root_path / ".package-upgrade.lock"
        recovery_path = root_path / ".package-upgrade-lock-recovery"
        if not lock_path.exists():
            return
        recovery_fd = os.open(recovery_path, os.O_CREAT | os.O_RDWR | getattr(os, "O_NOFOLLOW", 0), 0o600)
        try:
            fcntl.flock(recovery_fd, fcntl.LOCK_EX)
            owner_path = lock_path / "owner.json"
            if lock_path.is_symlink() or owner_path.is_symlink() or not owner_path.is_file():
                fail("cannot safely release package-upgrade lock")
            owner = json.loads(owner_path.read_bytes())
            if owner != {"schema_version": "oasis7.package_upgrade_lock_owner.v1", "pid": int(pid_text), "operation_id": operation_id}:
                fail("package-upgrade lock ownership changed before release")
            shutil.rmtree(lock_path)
            root_fd = os.open(root_path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
            try:
                os.fsync(root_fd)
            finally:
                os.close(root_fd)
        finally:
            fcntl.flock(recovery_fd, fcntl.LOCK_UN)
            os.close(recovery_fd)
        return
    fail(f"unknown operation transaction command: {command}")


main()
PY
}

require_safe_operation_id() {
  local flag=$1 value=$2
  [[ "$value" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ && "$value" != *..* ]] \
    || die "$flag must be a safe single path token"
}

operation_stage_exit() {
  local status=$?
  trap - EXIT
  if [[ -n "${operation_temp_dir:-}" && -e "$operation_temp_dir" ]]; then
    rm -rf -- "$operation_temp_dir"
  fi
  if [[ -n "${operation_lock_dir:-}" && -n "${node_root:-}" ]]; then
    operation_transaction_tool release-lock "$node_root" "$operation_id" "$$" >/dev/null 2>&1 || true
  fi
  exit "$status"
}

node_root=""
package_deb=""
ops_tools_tar=""
native_runtime=""
native_runtime_sha256=""
native_buildinfo=""
native_buildinfo_sha256=""
package_version=""
commit=""
run_id=""
artifact_ref=""
systemd_service=""
release_retention_count=3
restart_service=0
operation_promote=0
post_restart_status_url=""
post_restart_health_url=""
post_restart_timeout_secs=60
operation_mode=""
operation_id=""
action_id=""
operation_request_digest=""
operation_record_digest=""
operation_profile=""
operation_lifecycle_data_dir=""
expected_instance_id=""
expected_incarnation=""
expected_generation=""
expected_config_digest=""
expected_instance_action_id=""
promotion_action_id=""
stop_action_id=""
failed_start_action_id=""
failed_start_receipt_sha256=""
rollback_action_id=""
candidate_chain_plan=""
candidate_chain_plan_sha256=""
runtime_compatibility_plan=""
runtime_compatibility_plan_sha256=""

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
    --native-runtime)
      native_runtime=${2:-}
      shift 2
      ;;
    --native-runtime-sha256)
      native_runtime_sha256=${2:-}
      shift 2
      ;;
    --native-buildinfo)
      native_buildinfo=${2:-}
      shift 2
      ;;
    --native-buildinfo-sha256)
      native_buildinfo_sha256=${2:-}
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
    --operation-mode)
      operation_mode=${2:-}
      shift 2
      ;;
    --operation-id)
      operation_id=${2:-}
      shift 2
      ;;
    --action-id)
      action_id=${2:-}
      shift 2
      ;;
    --operation-request-digest)
      operation_request_digest=${2:-}
      shift 2
      ;;
    --operation-record-digest)
      operation_record_digest=${2:-}
      shift 2
      ;;
    --operation-profile)
      operation_profile=${2:-}
      shift 2
      ;;
    --lifecycle-data-dir)
      operation_lifecycle_data_dir=${2:-}
      shift 2
      ;;
    --expected-instance-id)
      expected_instance_id=${2:-}
      shift 2
      ;;
    --expected-incarnation)
      expected_incarnation=${2:-}
      shift 2
      ;;
    --expected-generation)
      expected_generation=${2:-}
      shift 2
      ;;
    --expected-config-digest)
      expected_config_digest=${2:-}
      shift 2
      ;;
    --expected-instance-action-id)
      expected_instance_action_id=${2:-}
      shift 2
      ;;
    --promotion-action-id)
      promotion_action_id=${2:-}
      shift 2
      ;;
    --stop-action-id)
      stop_action_id=${2:-}
      shift 2
      ;;
    --failed-start-action-id)
      failed_start_action_id=${2:-}
      shift 2
      ;;
    --failed-start-receipt-sha256)
      failed_start_receipt_sha256=${2:-}
      shift 2
      ;;
    --rollback-action-id)
      rollback_action_id=${2:-}
      shift 2
      ;;
    --candidate-chain-plan)
      candidate_chain_plan=${2:-}
      shift 2
      ;;
    --candidate-chain-plan-sha256)
      candidate_chain_plan_sha256=${2:-}
      shift 2
      ;;
    --runtime-compatibility-plan)
      runtime_compatibility_plan=${2:-}
      shift 2
      ;;
    --runtime-compatibility-plan-sha256)
      runtime_compatibility_plan_sha256=${2:-}
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

if [[ -n "$operation_mode" || -n "$operation_id" || -n "$action_id" || -n "$operation_request_digest" || -n "$operation_record_digest" || -n "$operation_profile" || -n "$operation_lifecycle_data_dir" || -n "$expected_instance_id" || -n "$expected_incarnation" || -n "$expected_generation" || -n "$expected_config_digest" || -n "$expected_instance_action_id" || -n "$promotion_action_id" || -n "$stop_action_id" || -n "$failed_start_action_id" || -n "$failed_start_receipt_sha256" || -n "$rollback_action_id" || -n "$native_runtime" || -n "$native_runtime_sha256" || -n "$native_buildinfo" || -n "$native_buildinfo_sha256" || -n "$candidate_chain_plan" || -n "$candidate_chain_plan_sha256" || -n "$runtime_compatibility_plan" || -n "$runtime_compatibility_plan_sha256" ]]; then
  [[ -n "$operation_mode" ]] || die "operation IDs require --operation-mode"
  require_non_empty "--operation-id" "$operation_id"
  require_non_empty "--action-id" "$action_id"
  require_safe_operation_id "--operation-id" "$operation_id"
  require_safe_operation_id "--action-id" "$action_id"
  if [[ -n "$operation_request_digest" && ! "$operation_request_digest" =~ ^[0-9a-f]{64}$ ]]; then
    die "--operation-request-digest must be 64 lowercase hexadecimal characters"
  fi
  if [[ -n "$operation_record_digest" && ! "$operation_record_digest" =~ ^[0-9a-f]{64}$ ]]; then
    die "--operation-record-digest must be 64 lowercase hexadecimal characters"
  fi

  case "$operation_mode" in
    query)
      [[ -z "$package_deb" && -z "$ops_tools_tar" && -z "$native_runtime" && -z "$native_runtime_sha256" && -z "$native_buildinfo" && -z "$native_buildinfo_sha256" && -z "$candidate_chain_plan" && -z "$candidate_chain_plan_sha256" && -z "$runtime_compatibility_plan" && -z "$runtime_compatibility_plan_sha256" && -z "$package_version" && -z "$commit" && -z "$run_id" && -z "$artifact_ref" && -z "$expected_instance_id" && -z "$expected_incarnation" && -z "$expected_generation" && -z "$expected_config_digest" && -z "$expected_instance_action_id" && -z "$operation_lifecycle_data_dir" && -z "$promotion_action_id" && -z "$stop_action_id" && -z "$failed_start_action_id" && -z "$failed_start_receipt_sha256" && -z "$rollback_action_id" && "$restart_service" -eq 0 && -z "$systemd_service" && -z "$post_restart_status_url" && -z "$post_restart_health_url" && "$release_retention_count" -eq 3 && "$post_restart_timeout_secs" -eq 60 ]] \
        || die "operation query accepts only node-root, operation/action IDs, profile, and immutable request digest"
      require_non_empty "--node-root" "$node_root"
      query_digest=${operation_record_digest:-$operation_request_digest}
      require_non_empty "--operation request digest" "$query_digest"
      if [[ -n "$operation_record_digest" && "$operation_profile" != "public_testnet_linux_package_v1" && "$operation_profile" != "local_native_package_v1" ]]; then
        die "--operation-record-digest requires a supported local package profile"
      fi
      if [[ -n "$operation_request_digest" && -n "$operation_record_digest" ]]; then
        die "operation query accepts exactly one request digest"
      fi
      [[ -d "$node_root" ]] || die "missing node root: $node_root"
      node_root=$(abs_path "$node_root")
      if [[ -n "$operation_profile" ]]; then
        operation_lock_dir="$node_root/.package-upgrade.lock"
        operation_transaction_tool acquire-lock "$node_root" "$operation_id" "$$"
        trap operation_stage_exit EXIT
      fi
      operation_transaction_tool query "$node_root" "$operation_id" "$action_id" "$query_digest"
      exit 0
      ;;
    promote)
      [[ -z "$failed_start_action_id" && -z "$failed_start_receipt_sha256" && -z "$rollback_action_id" && -z "$runtime_compatibility_plan" && -z "$runtime_compatibility_plan_sha256" ]] || die "promotion does not accept rollback or stage-plan fields"
      require_non_empty "--node-root" "$node_root"
      if [[ -z "$operation_profile" ]]; then
        die "exact stopped-instance evidence is not available"
      fi
      require_non_empty "--operation-profile" "$operation_profile"
      require_non_empty "--operation-record-digest" "$operation_record_digest"
      require_non_empty "--operation-request-digest" "$operation_request_digest"
      require_non_empty "--lifecycle-data-dir" "$operation_lifecycle_data_dir"
      require_non_empty "--expected-incarnation" "$expected_incarnation"
      require_non_empty "--expected-generation" "$expected_generation"
      require_non_empty "--expected-config-digest" "$expected_config_digest"
      require_non_empty "--expected-instance-action-id" "$expected_instance_action_id"
      require_non_empty "--promotion-action-id" "$promotion_action_id"
      require_non_empty "--stop-action-id" "$stop_action_id"
      [[ "$operation_profile" == "public_testnet_linux_package_v1" || "$operation_profile" == "local_native_package_v1" ]] || die "unsupported package deployment profile"
      [[ "$operation_request_digest" =~ ^[0-9a-f]{64}$ && "$expected_config_digest" =~ ^[0-9a-f]{64}$ ]] || die "promotion request digests must be canonical SHA256"
      [[ "$expected_generation" =~ ^[0-9]+$ ]] || die "--expected-generation must be a non-negative integer"
      [[ -d "$node_root" && -d "$operation_lifecycle_data_dir" ]] || die "package promotion requires exact local node/lifecycle directories"
      node_root=$(abs_path "$node_root")
      operation_lifecycle_data_dir=$(abs_path "$operation_lifecycle_data_dir")
      operation_lock_dir="$node_root/.package-upgrade.lock"
      operation_transaction_tool acquire-lock "$node_root" "$operation_id" "$$"
      trap operation_stage_exit EXIT
      operation_transaction_tool promote \
        "$node_root" "$operation_id" "$action_id" "$operation_record_digest" \
        "$operation_request_digest" "$operation_lifecycle_data_dir" \
        "$expected_incarnation" "$expected_generation" "$expected_config_digest" \
        "$expected_instance_action_id" "$promotion_action_id" "$stop_action_id" \
        "$operation_profile"
      exit 0
      ;;
    rollback)
      [[ -z "$package_deb" && -z "$ops_tools_tar" && -z "$native_runtime" && -z "$native_runtime_sha256" && -z "$native_buildinfo" && -z "$native_buildinfo_sha256" && -z "$candidate_chain_plan" && -z "$candidate_chain_plan_sha256" && -z "$package_version" && -z "$commit" && -z "$run_id" && -z "$artifact_ref" && "$restart_service" -eq 0 && -z "$systemd_service" && -z "$post_restart_status_url" && -z "$post_restart_health_url" && "$release_retention_count" -eq 3 && "$post_restart_timeout_secs" -eq 60 ]] || die "operation rollback accepts only exact operation, owner, and failed-start evidence"
      require_non_empty "--node-root" "$node_root"
      require_non_empty "--operation-profile" "$operation_profile"
      require_non_empty "--operation-record-digest" "$operation_record_digest"
      require_non_empty "--operation-request-digest" "$operation_request_digest"
      require_non_empty "--lifecycle-data-dir" "$operation_lifecycle_data_dir"
      require_non_empty "--expected-incarnation" "$expected_incarnation"
      require_non_empty "--expected-generation" "$expected_generation"
      require_non_empty "--expected-config-digest" "$expected_config_digest"
      require_non_empty "--expected-instance-action-id" "$expected_instance_action_id"
      [[ "$expected_instance_id" == "$expected_incarnation" ]] || die "rollback instance ID and incarnation must match"
      require_non_empty "--promotion-action-id" "$promotion_action_id"
      require_non_empty "--stop-action-id" "$stop_action_id"
      require_non_empty "--failed-start-action-id" "$failed_start_action_id"
      require_non_empty "--failed-start-receipt-sha256" "$failed_start_receipt_sha256"
      require_non_empty "--rollback-action-id" "$rollback_action_id"
      [[ "$operation_profile" == "public_testnet_linux_package_v1" || "$operation_profile" == "local_native_package_v1" ]] || die "unsupported package deployment profile"
      [[ -n "$runtime_compatibility_plan" && "$runtime_compatibility_plan_sha256" =~ ^[0-9a-f]{64}$ ]] || die "operation rollback requires the immutable runtime compatibility plan"
      [[ "$operation_request_digest" =~ ^[0-9a-f]{64}$ && "$operation_record_digest" =~ ^[0-9a-f]{64}$ && "$expected_config_digest" =~ ^[0-9a-f]{64}$ && "$failed_start_receipt_sha256" =~ ^[0-9a-f]{64}$ ]] || die "rollback request and receipt digests must be canonical SHA256"
      [[ "$expected_generation" =~ ^[0-9]+$ ]] || die "--expected-generation must be a non-negative integer"
      [[ -d "$node_root" && -d "$operation_lifecycle_data_dir" ]] || die "package rollback requires exact local node/lifecycle directories"
      node_root=$(abs_path "$node_root")
      operation_lifecycle_data_dir=$(abs_path "$operation_lifecycle_data_dir")
      operation_lock_dir="$node_root/.package-upgrade.lock"
      operation_transaction_tool acquire-lock "$node_root" "$operation_id" "$$"
      trap operation_stage_exit EXIT
      operation_transaction_tool rollback \
        "$node_root" "$operation_id" "$action_id" "$operation_record_digest" \
        "$operation_request_digest" "$operation_lifecycle_data_dir" \
        "$expected_incarnation" "$expected_generation" "$expected_config_digest" \
        "$expected_instance_action_id" "$promotion_action_id" "$stop_action_id" \
        "$failed_start_action_id" "$failed_start_receipt_sha256" "$rollback_action_id" \
        "$operation_profile" "$runtime_compatibility_plan" "$runtime_compatibility_plan_sha256"
      exit 0
      ;;
    stage)
      require_non_empty "--node-root" "$node_root"
      if [[ "$operation_profile" == "local_native_package_v1" ]]; then
        require_non_empty "--native-runtime" "$native_runtime"
        require_non_empty "--native-runtime-sha256" "$native_runtime_sha256"
        require_non_empty "--native-buildinfo" "$native_buildinfo"
        require_non_empty "--native-buildinfo-sha256" "$native_buildinfo_sha256"
        [[ -z "$package_deb" && -z "$ops_tools_tar" ]] || die "local native package cannot include Linux package inputs"
        [[ "$native_runtime_sha256" =~ ^[0-9a-f]{64}$ && "$native_buildinfo_sha256" =~ ^[0-9a-f]{64}$ ]] || die "native package inputs must use canonical SHA256"
        [[ -n "$candidate_chain_plan" && -n "$candidate_chain_plan_sha256" || -z "$candidate_chain_plan" && -z "$candidate_chain_plan_sha256" ]] || die "candidate-chain plan path and digest must be supplied together"
        [[ -z "$candidate_chain_plan_sha256" || "$candidate_chain_plan_sha256" =~ ^[0-9a-f]{64}$ ]] || die "candidate-chain plan digest must be canonical SHA256"
      else
        require_non_empty "--package-deb" "$package_deb"
        require_non_empty "--ops-tools-tar" "$ops_tools_tar"
        [[ -z "$native_runtime" && -z "$native_runtime_sha256" && -z "$native_buildinfo" && -z "$native_buildinfo_sha256" ]] || die "Linux package cannot include local native inputs"
        [[ -z "$candidate_chain_plan" && -z "$candidate_chain_plan_sha256" ]] || die "Linux package cannot include a local candidate-chain plan"
      fi
      require_non_empty "--package-version" "$package_version"
      require_safe_package_version "$package_version"
      require_non_empty "--commit" "$commit"
      require_non_empty "--run-id" "$run_id"
      require_non_empty "--expected-instance-id" "$expected_instance_id"
      require_non_empty "--expected-generation" "$expected_generation"
      if [[ -n "$operation_profile" ]]; then
        [[ "$operation_profile" == "public_testnet_linux_package_v1" || "$operation_profile" == "local_native_package_v1" ]] || die "unsupported package deployment profile"
        require_non_empty "--operation-record-digest" "$operation_record_digest"
        require_non_empty "--lifecycle-data-dir" "$operation_lifecycle_data_dir"
        require_non_empty "--expected-incarnation" "$expected_incarnation"
        require_non_empty "--expected-config-digest" "$expected_config_digest"
        require_non_empty "--expected-instance-action-id" "$expected_instance_action_id"
        [[ "$operation_record_digest" =~ ^[0-9a-f]{64}$ && "$expected_config_digest" =~ ^[0-9a-f]{64}$ ]] || die "operation/lifecycle digests must be canonical SHA256"
        [[ "$expected_incarnation" == "$expected_instance_id" ]] || die "expected instance ID and incarnation must match"
        [[ -n "$runtime_compatibility_plan" && "$runtime_compatibility_plan_sha256" =~ ^[0-9a-f]{64}$ ]] \
          || die "operation-mode package stage requires an immutable runtime compatibility plan"
      elif [[ -n "$runtime_compatibility_plan" || -n "$runtime_compatibility_plan_sha256" ]]; then
        die "runtime compatibility plans require an OperationRecord-bound package stage"
      fi
      [[ "$expected_instance_id" =~ ^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$ && "$expected_instance_id" != *..* ]] \
        || die "--expected-instance-id must be a safe identity token"
      [[ "$expected_generation" =~ ^[0-9]+$ ]] || die "--expected-generation must be a non-negative integer"
      [[ -d "$node_root" ]] || die "missing node root: $node_root"
      if [[ -n "$operation_lifecycle_data_dir" ]]; then
        [[ -d "$operation_lifecycle_data_dir" ]] || die "missing lifecycle data directory"
        operation_lifecycle_data_dir=$(abs_path "$operation_lifecycle_data_dir")
      fi
      if [[ "$operation_profile" == "local_native_package_v1" ]]; then
        [[ -f "$native_runtime" && -x "$native_runtime" ]] || die "missing executable local native runtime: $native_runtime"
        [[ -f "$native_buildinfo" ]] || die "missing local native BUILDINFO: $native_buildinfo"
        native_runtime=$(abs_path "$native_runtime")
        native_buildinfo=$(abs_path "$native_buildinfo")
        if [[ -n "$candidate_chain_plan" ]]; then
          [[ -f "$candidate_chain_plan" && ! -L "$candidate_chain_plan" ]] || die "missing regular candidate-chain plan: $candidate_chain_plan"
          candidate_chain_plan=$(abs_path "$candidate_chain_plan")
          [[ "$(sha256_file "$candidate_chain_plan")" == "$candidate_chain_plan_sha256" ]] || die "candidate-chain plan digest mismatch"
        fi
        [[ "$(sha256_file "$native_runtime")" == "$native_runtime_sha256" ]] || die "local native runtime digest mismatch"
        [[ "$(sha256_file "$native_buildinfo")" == "$native_buildinfo_sha256" ]] || die "local native BUILDINFO digest mismatch"
        stage_package_source="$native_runtime"
        stage_ops_source="$native_buildinfo"
      else
        [[ -f "$package_deb" ]] || die "missing Debian package: $package_deb"
        [[ -f "$ops_tools_tar" ]] || die "missing ops-tools package: $ops_tools_tar"
        package_deb=$(abs_path "$package_deb")
        ops_tools_tar=$(abs_path "$ops_tools_tar")
        stage_package_source="$package_deb"
        stage_ops_source="$ops_tools_tar"
      fi
      [[ "$restart_service" -eq 0 && -z "$systemd_service" && -z "$post_restart_status_url" && -z "$post_restart_health_url" ]] \
        || die "operation stage cannot stop, start, or probe a service"
      [[ "$release_retention_count" -eq 3 && "$post_restart_timeout_secs" -eq 60 ]] \
        || die "operation stage cannot change release retention or service-probe behavior"
      node_root=$(abs_path "$node_root")
      if [[ "$operation_profile" == "local_native_package_v1" ]]; then
        artifact_ref=${artifact_ref:-"local-native-runtime-file"}
      else
        artifact_ref=${artifact_ref:-"oasis7-linux-x64.deb!/opt/oasis7/bin/oasis7_chain_runtime"}
      fi
      operation_parent="$node_root/package-upgrade-operations"
      operation_record="$operation_parent/$operation_id"
      operation_temp_dir="$operation_parent/.$operation_id.tmp"
      operation_lock_dir="$node_root/.package-upgrade.lock"
      operation_transaction_tool acquire-lock "$node_root" "$operation_id" "$$"
      trap operation_stage_exit EXIT
      if [[ -n "$operation_profile" ]]; then
        operation_transaction_tool validate-current "$node_root" "$operation_profile" >/dev/null
      fi
      operation_transaction_tool prepare-parent "$operation_parent" "$operation_temp_dir"

      if [[ -e "$operation_record" || -L "$operation_record" ]]; then
        operation_transaction_tool retry \
          "$node_root" "$operation_id" "$action_id" "$operation_request_digest" \
          "$stage_package_source" "$stage_ops_source" "$expected_instance_id" "$expected_generation" \
          "$package_version" "$commit" "$run_id" "$artifact_ref" \
          "$operation_profile" "$operation_record_digest" "$expected_incarnation" \
          "$expected_config_digest" "$expected_instance_action_id" "$operation_lifecycle_data_dir"
        exit 0
      fi

      umask 077
      mkdir -p "$operation_temp_dir/artifacts" "$operation_temp_dir/work" "$operation_temp_dir/prepared"
      cp "$stage_package_source" "$operation_temp_dir/artifacts/package.deb"
      cp "$stage_ops_source" "$operation_temp_dir/artifacts/ops-tools.tar.gz"
      chmod 600 "$operation_temp_dir/artifacts/package.deb" "$operation_temp_dir/artifacts/ops-tools.tar.gz"
      if [[ "$operation_profile" == "local_native_package_v1" ]]; then
        python3 - "$operation_temp_dir/artifacts/package.deb" "$operation_temp_dir/artifacts/ops-tools.tar.gz" "$operation_temp_dir/prepared/oasis7-linux-x64" "$package_version" "$commit" "$run_id" "$native_runtime_sha256" "$operation_temp_dir/artifacts/ops-tools.manifest" <<'PY'
from __future__ import annotations

import hashlib
import json
import os
import shutil
import stat
import sys
from pathlib import Path

runtime_source, buildinfo_source, prepared_text, version, commit, run_id, expected_runtime_sha, manifest_path = sys.argv[1:]
runtime_source = Path(runtime_source)
buildinfo_source = Path(buildinfo_source)
prepared = Path(prepared_text)
for source in (runtime_source, buildinfo_source):
    metadata = source.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise SystemExit(f"local native package input is not a regular file: {source}")
prepared.mkdir(mode=0o700, parents=True)
(prepared / "bin").mkdir(mode=0o755)
runtime = prepared / "bin" / "oasis7_chain_runtime"
shutil.copy2(runtime_source, runtime)
os.chmod(runtime, stat.S_IMODE(runtime.stat().st_mode) | 0o111)
shutil.copy2(buildinfo_source, prepared / "BUILDINFO")
actual_runtime_sha = hashlib.sha256(runtime.read_bytes()).hexdigest()
if actual_runtime_sha != expected_runtime_sha:
    raise SystemExit("staged local native runtime digest mismatch")
fields = {}
for line in (prepared / "BUILDINFO").read_text(encoding="utf-8").splitlines():
    if "=" in line:
        key, value = line.split("=", 1)
        fields[key] = value
if fields.get("package_version") != version or fields.get("commit") != commit or fields.get("run_id") != run_id or fields.get("platform") != "local-native" or fields.get("runtime_sha256") != expected_runtime_sha:
    raise SystemExit("local native BUILDINFO does not bind the requested package identity and runtime hash")
manifest = {
    "schema_version": "oasis7.local_native_package_artifact_manifest.v1",
    "package_version": version,
    "commit": commit,
    "run_id": run_id,
    "runtime_sha256": actual_runtime_sha,
    "buildinfo_sha256": hashlib.sha256((prepared / "BUILDINFO").read_bytes()).hexdigest(),
}
Path(manifest_path).write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
PY
        chmod 600 "$operation_temp_dir/artifacts/ops-tools.manifest"
      else
      command -v dpkg-deb >/dev/null 2>&1 || die "dpkg-deb is required to extract the Debian package"
      safe_deb_extract "$operation_temp_dir/artifacts/package.deb" "$operation_temp_dir/work/deb-root"
      bundle_root="$operation_temp_dir/work/deb-root/opt/oasis7"
      [[ -d "$bundle_root" ]] || die "Debian package missing /opt/oasis7 player bundle: $package_deb"
      python3 "$SCRIPT_DIR/p2p-verify-linux-package-bundle.py" \
        "$bundle_root" "$package_version" "$commit" "$run_id" \
        || die "embedded Debian BUILDINFO/SHA256SUMS provenance verification failed"
      safe_ops_tools_extract "$operation_temp_dir/artifacts/ops-tools.tar.gz" "$operation_temp_dir/work"
      ops_bundle_root="$operation_temp_dir/work/oasis7-linux-x64-ops-tools"
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
      for ops_binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit service-readback; do
        [[ -x "$ops_bundle_root/bin/$ops_binary" ]] || die "ops-tools archive missing executable: $ops_binary"
      done
      cp "$ops_bundle_root/.oasis7-ops-tools-manifest.json" "$operation_temp_dir/artifacts/ops-tools.manifest"
      chmod 600 "$operation_temp_dir/artifacts/ops-tools.manifest"
      mkdir -p "$bundle_root/bin"
      cp -a "$ops_bundle_root/bin/." "$bundle_root/bin/"
      python3 "$SCRIPT_DIR/p2p-rebuild-linux-bundle-checksums.py" "$bundle_root" \
        || die "deployed player/ops checksum closure rebuild failed"
      [[ -x "$bundle_root/bin/oasis7_chain_runtime" ]] || die "bundle missing executable runtime: $bundle_root/bin/oasis7_chain_runtime"
      ensure_governed_bootstrap_bundle_exists "$node_root"
      mv "$bundle_root" "$operation_temp_dir/prepared/oasis7-linux-x64"
      rm -rf "$operation_temp_dir/work"
      fi
      request_args=(
        "$node_root" "$operation_id" "$action_id" "$expected_instance_id" "$expected_generation" \
        "$package_version" "$commit" "$run_id" "$artifact_ref" \
        "$operation_temp_dir/artifacts" "$operation_temp_dir/prepared/oasis7-linux-x64" \
        "$operation_profile" "$operation_record_digest" "$expected_incarnation" \
        "$expected_config_digest" "$expected_instance_action_id" "$operation_lifecycle_data_dir"
      )
      if [[ -n "$candidate_chain_plan" ]]; then
        request_args+=("$candidate_chain_plan" "$candidate_chain_plan_sha256")
      elif [[ -n "$operation_profile" ]]; then
        request_args+=("" "")
      fi
      if [[ -n "$operation_profile" ]]; then
        request_args+=("$runtime_compatibility_plan" "$runtime_compatibility_plan_sha256")
      fi
      operation_request_json="$(operation_transaction_tool request "${request_args[@]}")"
      operation_request_digest_computed="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(sys.argv[1].encode("utf-8")).hexdigest())' "$operation_request_json")"
      if [[ -n "$operation_request_digest" && "$operation_request_digest" != "$operation_request_digest_computed" ]]; then
        die "computed immutable operation request digest does not match --operation-request-digest"
      fi
      operation_transaction_tool persist \
        "$operation_temp_dir" "$operation_record" "$operation_request_json" "$operation_request_digest_computed"
      exit 0
      ;;
    *)
      die "--operation-mode must be stage, query, or promote"
      ;;
  esac
fi

require_non_empty "--node-root" "$node_root"
require_non_empty "--package-deb" "$package_deb"
require_non_empty "--ops-tools-tar" "$ops_tools_tar"
require_non_empty "--package-version" "$package_version"
require_safe_package_version "$package_version"
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
for ops_binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit service-readback; do
  [[ -x "$ops_bundle_root/bin/$ops_binary" ]] || die "ops-tools archive missing executable: $ops_binary"
done
mkdir -p "$bundle_root/bin"
cp -a "$ops_bundle_root/bin/." "$bundle_root/bin/"
python3 "$SCRIPT_DIR/p2p-rebuild-linux-bundle-checksums.py" "$bundle_root" \
  || die "deployed player/ops checksum closure rebuild failed"
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
