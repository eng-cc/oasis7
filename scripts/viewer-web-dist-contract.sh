#!/usr/bin/env bash

viewer_web_dist_contract_pairs() {
  cat <<'EOF'
software_safe.html index.html
software_safe.html viewer.html
software_safe.html software_safe.html
viewer.js viewer.js
software_safe.js software_safe.js
software_safe_first_agent_claim_evidence.html software_safe_first_agent_claim_evidence.html
favicon.ico favicon.ico
EOF
}

viewer_web_dist_manifest_name() {
  printf '%s\n' '.oasis7-viewer-dist-manifest.json'
}

viewer_web_dist_source_metadata_json() {
  local repo_root=$1
  python3 - "$repo_root" <<'PY'
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
scope = [
    "Cargo.toml",
    "Cargo.lock",
    "crates/oasis7_viewer/software_safe.html",
    "crates/oasis7_viewer/viewer.js",
    "crates/oasis7_viewer/software_safe.js",
    "crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html",
    "crates/oasis7_viewer/package.json",
    "crates/oasis7_viewer/package-lock.json",
    "crates/oasis7_viewer/vite.software-safe.config.mjs",
    "crates/oasis7_viewer/scripts",
    "crates/oasis7_viewer/software_safe_src",
    "crates/oasis7_viewer/pixel-world-bridge",
    "crates/oasis7_viewer/favicon.ico",
    "crates/pixel_world_bridge/Cargo.toml",
    "crates/pixel_world_bridge/src",
    "crates/oasis7_proto/Cargo.toml",
    "crates/oasis7_proto/src",
    "scripts/copy-viewer-web-dist.sh",
    "scripts/viewer-web-dist-contract.sh",
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

print(json.dumps(
    {
        "sourceFingerprint": hasher.hexdigest(),
        "sourceFileCount": len(unique_files),
        "sourceLatestPath": latest_rel,
        "sourceLatestMtimeNs": latest_mtime_ns,
    },
    ensure_ascii=False,
))
PY
}

viewer_web_dist_write_manifest() {
  local repo_root=$1
  local dist_dir=$2
  local metadata_json
  metadata_json=$(viewer_web_dist_source_metadata_json "$repo_root")
  python3 - "$metadata_json" "$dist_dir" "$(viewer_web_dist_manifest_name)" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

source_metadata = json.loads(sys.argv[1])
dist_dir = Path(sys.argv[2]).resolve()
manifest_name = sys.argv[3]
manifest_path = dist_dir / manifest_name

dist_files: dict[str, dict[str, int]] = {}
for candidate in sorted(dist_dir.rglob("*")):
    if not candidate.is_file():
        continue
    if candidate == manifest_path:
        continue
    rel = candidate.relative_to(dist_dir).as_posix()
    dist_files[rel] = {"size": candidate.stat().st_size}

manifest = {
    **source_metadata,
    "distFiles": dist_files,
}
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}
