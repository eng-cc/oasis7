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
    [--restart-service]

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

node_root=""
bundle_tar=""
package_version=""
commit=""
run_id=""
artifact_ref=""
systemd_service=""
restart_service=0

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
    --restart-service)
      restart_service=1
      shift
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

node_root=$(abs_path "$node_root")
bundle_tar=$(abs_path "$bundle_tar")
artifact_ref=${artifact_ref:-"oasis7-linux-x64-bundle.tar.gz!/bin/oasis7_chain_runtime"}

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
if [[ -L "$current_path" || -e "$current_path" ]]; then
  readlink -f "$current_path" >"$node_root/last-$backup_suffix.txt" || true
  if [[ -d "$current_path" && ! -L "$current_path" ]]; then
    mv "$current_path" "$node_root/current-$backup_suffix.dir"
  else
    rm -f "$current_path"
  fi
fi
ln -s "$release_dir" "$current_path"

if [[ "$restart_service" -eq 1 ]]; then
  systemctl daemon-reload
  systemctl restart "$systemd_service"
  sleep 3
  systemctl is-active --quiet "$systemd_service"
  systemctl --no-pager --full status "$systemd_service" | sed -n '1,18p'
fi

echo "upgraded $node_root to $package_version"
