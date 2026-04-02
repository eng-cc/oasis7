#!/usr/bin/env bash

bundle_manifest_filename() {
  printf '%s\n' '.oasis7-bundle-manifest.json'
}

bundle_manifest_path() {
  local bundle_dir=$1
  printf '%s/%s\n' "$bundle_dir" "$(bundle_manifest_filename)"
}

bundle_source_metadata_json() {
  local repo_root=$1
  python3 - "$repo_root" <<'PY'
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
scope = [
    "Cargo.lock",
    "crates/oasis7/Cargo.toml",
    "crates/oasis7/src/viewer",
    "crates/oasis7/src/bin/oasis7_game_launcher.rs",
    "crates/oasis7/src/bin/oasis7_game_launcher",
    "crates/oasis7/src/bin/oasis7_web_launcher.rs",
    "crates/oasis7/src/bin/oasis7_web_launcher",
    "crates/oasis7_proto/Cargo.toml",
    "crates/oasis7_proto/src",
    "crates/oasis7_viewer/Cargo.toml",
    "crates/oasis7_viewer/Trunk.toml",
    "crates/oasis7_viewer/index.html",
    "crates/oasis7_viewer/software_safe.html",
    "crates/oasis7_viewer/software_safe.js",
    "crates/oasis7_viewer/package.json",
    "crates/oasis7_viewer/package-lock.json",
    "crates/oasis7_viewer/vite.software-safe.config.mjs",
    "crates/oasis7_viewer/scripts",
    "crates/oasis7_viewer/software_safe_src",
    "crates/oasis7_viewer/favicon.ico",
    "crates/oasis7_viewer/src",
    "crates/oasis7_viewer/assets",
    "crates/oasis7_client_launcher/Cargo.toml",
    "crates/oasis7_client_launcher/Trunk.toml",
    "crates/oasis7_client_launcher/index.html",
    "crates/oasis7_client_launcher/src",
]
files: list[Path] = []
for entry in scope:
    path = repo_root / entry
    if path.is_dir():
        files.extend(sorted(candidate for candidate in path.rglob("*") if candidate.is_file()))
    elif path.is_file():
        files.append(path)

unique_files = sorted({candidate.resolve() for candidate in files})
hasher = hashlib.sha256()
latest_mtime_ns = 0
latest_rel = ""
for candidate in unique_files:
    rel = candidate.relative_to(repo_root).as_posix()
    hasher.update(rel.encode("utf-8"))
    hasher.update(b"\0")
    hasher.update(candidate.read_bytes())
    hasher.update(b"\0")
    stat = candidate.stat()
    if stat.st_mtime_ns >= latest_mtime_ns:
        latest_mtime_ns = stat.st_mtime_ns
        latest_rel = rel

viewer_proto = (repo_root / "crates/oasis7_proto/src/viewer.rs").read_text(encoding="utf-8")
match = re.search(r"pub const VIEWER_PROTOCOL_VERSION: u32 = (\d+);", viewer_proto)
viewer_protocol_version = int(match.group(1)) if match else None

print(json.dumps(
    {
        "sourceFingerprint": hasher.hexdigest(),
        "sourceFileCount": len(unique_files),
        "sourceLatestPath": latest_rel,
        "sourceLatestMtimeNs": latest_mtime_ns,
        "viewerProtocolVersion": viewer_protocol_version,
        "sourceScope": scope,
    },
    ensure_ascii=False,
))
PY
}

bundle_write_manifest() {
  local repo_root=$1
  local bundle_dir=$2
  local metadata_json manifest_path
  metadata_json=$(bundle_source_metadata_json "$repo_root")
  manifest_path=$(bundle_manifest_path "$bundle_dir")
  python3 - "$metadata_json" "$bundle_dir" "$manifest_path" <<'PY'
from __future__ import annotations

import hashlib
import json
import time
import sys
from pathlib import Path

source_metadata = json.loads(sys.argv[1])
bundle_dir = Path(sys.argv[2]).resolve()
manifest_path = Path(sys.argv[3]).resolve()


def hash_first(pattern: str) -> tuple[str | None, str | None]:
    matches = sorted(bundle_dir.glob(pattern))
    if not matches:
        return None, None
    candidate = matches[0]
    digest = hashlib.sha256(candidate.read_bytes()).hexdigest()
    return candidate.relative_to(bundle_dir).as_posix(), digest

viewer_js_path, viewer_js_sha256 = hash_first("web/*.js")
viewer_wasm_path, viewer_wasm_sha256 = hash_first("web/*.wasm")
launcher_js_path, launcher_js_sha256 = hash_first("web-launcher/*.js")
launcher_wasm_path, launcher_wasm_sha256 = hash_first("web-launcher/*.wasm")

manifest = {
    "bundleSchemaVersion": 1,
    "generatedAtUnixMs": int(time.time() * 1000),
    **source_metadata,
    "assets": {
        "viewerJsPath": viewer_js_path,
        "viewerJsSha256": viewer_js_sha256,
        "viewerWasmPath": viewer_wasm_path,
        "viewerWasmSha256": viewer_wasm_sha256,
        "launcherJsPath": launcher_js_path,
        "launcherJsSha256": launcher_js_sha256,
        "launcherWasmPath": launcher_wasm_path,
        "launcherWasmSha256": launcher_wasm_sha256,
    },
}
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}

bundle_check_freshness() {
  local repo_root=$1
  local bundle_dir=$2
  local manifest_path current_json
  manifest_path=$(bundle_manifest_path "$bundle_dir")
  if [[ ! -f "$manifest_path" ]]; then
    echo "bundle freshness manifest missing: $manifest_path" >&2
    return 1
  fi
  current_json=$(bundle_source_metadata_json "$repo_root")
  python3 - "$manifest_path" "$current_json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
current = json.loads(sys.argv[2])
errors: list[str] = []
if manifest.get("sourceFingerprint") != current.get("sourceFingerprint"):
    errors.append(
        "source fingerprint drift "
        f"(bundle latest={manifest.get('sourceLatestPath')}, current latest={current.get('sourceLatestPath')})"
    )
if manifest.get("viewerProtocolVersion") != current.get("viewerProtocolVersion"):
    errors.append(
        "viewer protocol version drift "
        f"(bundle={manifest.get('viewerProtocolVersion')}, current={current.get('viewerProtocolVersion')})"
    )
if errors:
    print("bundle is stale relative to current workspace: " + "; ".join(errors))
    sys.exit(1)
print("bundle freshness ok")
PY
}
