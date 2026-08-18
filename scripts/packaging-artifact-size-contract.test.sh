#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

python3 - \
  "$ROOT_DIR/.github/workflows/testnet-packages.yml" \
  "$ROOT_DIR/.github/workflows/mainnet-packages.yml" \
  "$ROOT_DIR/.github/workflows/release-packages.yml" \
  "$ROOT_DIR/scripts/release-prepare-bundle.sh" \
  "$ROOT_DIR/scripts/build-game-launcher-bundle.sh" \
  "$ROOT_DIR/scripts/build-viewer-software-safe.sh" \
  "$ROOT_DIR/scripts/copy-viewer-web-dist.sh" \
  "$ROOT_DIR/Cargo.toml" <<'PY'
from pathlib import Path
import re
import sys

testnet, mainnet, release, prepare, build_bundle, build_viewer, copy_viewer, root_cargo = map(
    lambda value: Path(value).read_text(encoding="utf-8"), sys.argv[1:]
)

failures: list[str] = []


def require(condition: bool, message: str) -> None:
    if not condition:
        failures.append(message)

# 1. Linux publishes one supported installer: the Debian package.  AppImage,
# secondary .deb, and the raw tar archive must not remain release outputs.
for name, workflow in (("testnet", testnet), ("mainnet", mainnet), ("release", release)):
    linux_entries = re.findall(
        r'"platform":"linux-x64".*?"asset_name":"([^"]+)"', workflow
    )
    if name == "release":
        linux_entries = ["oasis7-linux-x64.deb"] if "platform: linux-x64" in workflow else []
    require(linux_entries, f"{name}: missing linux-x64 package matrix entry")
    require(
        all(asset.endswith(".deb") for asset in linux_entries),
        f"{name}: Linux release matrix must publish only .deb, got {linux_entries}",
    )
    require("AppImage" not in workflow, f"{name}: AppImage must not be published")
    require(
        "Package secondary Linux .deb" not in workflow,
        f"{name}: secondary Linux .deb step duplicates the sole package",
    )
    require(
        "oasis7-linux-x64-bundle.tar.gz" not in workflow,
        f"{name}: raw Linux tar archive must not be a release artifact",
    )

# 2. Keep operations/recovery tools out of the player bundle while producing a
# separately checksummed ops bundle.  The names are the existing native tools;
# the explicit player/ops split is the production implementation contract.
ops_tools = (
    "oasis7_world_repair_rebuild",
    "oasis7_governance_registry_import",
    "oasis7_governance_registry_audit",
)
for tool in ops_tools:
    require(tool in build_bundle, f"missing existing ops tool reference: {tool}")
require(
    re.search(r"ops[-_ ]?(?:bundle|out|dir|tools)", prepare, re.I),
    "release bundle preparation must expose a distinct ops output",
)
require(
    re.search(r"player[-_ ]?(?:bundle|out|dir)", prepare, re.I),
    "release bundle preparation must name the player output explicitly",
)
require(
    re.search(r"SHA256SUMS|checksum", prepare, re.I),
    "ops bundle must carry a manifest/checksum contract",
)
require(
    re.search(r"ops[-_ ]?(?:bundle|artifact|package)", testnet + mainnet, re.I),
    "package workflows must publish the separate ops artifact",
)

# 2b. The large viewer WASM is published as an independent optional payload;
# package-native jobs consume only the web dist artifact and must not fold the
# payload back into player installers.
for name, workflow in (("testnet", testnet), ("mainnet", mainnet), ("release", release)):
    require(
        "--optional-payload-dir" in workflow,
        f"{name}: web build must stage the optional viewer payload",
    )
    require(
        re.search(rf"{name}-viewer-optional-payload", workflow),
        f"{name}: optional viewer payload must be uploaded separately",
    )
    web_upload = re.search(
        rf"name: {name}-web-dists(?P<body>[\s\S]*?)(?=\n\s*- name:|\Z)",
        workflow,
    )
    require(web_upload is not None, f"{name}: missing player web dist upload block")
    require(
        web_upload is None or "viewer-optional-payload" not in web_upload.group("body"),
        f"{name}: player web dist artifact must not include optional payload directory",
    )
require(
    "OPS_OUT_DIR" in build_bundle
    and "OPS_BIN_DIR" in build_bundle
    and re.search(
        r'(?s)replace_file "\$WORLD_REPAIR_REBUILD_SRC" "\$OPS_BIN_DIR/',
        build_bundle,
    ),
    "player bundle must route the world repair tool only to the ops output",
)
require(
    re.search(r"(?is)ops.*?(?:SHA256SUMS|checksum)", testnet + mainnet + release + prepare),
    "ops package must publish a manifest/checksum alongside its payload",
)

# 3. Release profile should strip symbols and use thin LTO/codegen tuning, but
# retain normal unwind/panic diagnostics unless explicitly changed later.
release_profile = re.search(
    r"(?ms)^\[profile\.release\]\s*(?P<body>.*?)(?=^\[|\Z)", root_cargo
)
require(release_profile, "root Cargo.toml must define an explicit release profile")
body = release_profile.group("body") if release_profile else ""
require(
    re.search(r"(?m)^\s*strip\s*=\s*[\"']symbols[\"']", body),
    "release profile must strip symbols",
)
require(
    re.search(r"(?m)^\s*lto\s*=\s*[\"']thin[\"']", body),
    "release profile must enable thin LTO",
)
require(
    re.search(r"(?m)^\s*codegen-units\s*=\s*1\s*(?:#.*)?$", body),
    "release profile must tune codegen-units to 1",
)
require(
    not re.search(r"(?m)^\s*panic\s*=", body),
    "release profile must not change panic strategy in this size-only slice",
)

# 4. The large pixel-world bridge WASM is an optional, separately staged
# payload.  A missing payload has to be represented deterministically instead
# of making the whole viewer copy fail or silently claiming it is present.
require(
    re.search(r"optional", copy_viewer, re.I),
    "viewer copy contract must identify the pixel-world WASM as optional",
)
require(
    re.search(r"stage|staged|separate", copy_viewer, re.I),
    "viewer copy contract must expose separate WASM staging",
)
require(
    re.search(r"missing|absent|not found", copy_viewer, re.I),
    "viewer copy contract must define deterministic missing-WASM behavior",
)
require(
    "pixel_world_bridge_bindgen_bg.wasm" in copy_viewer,
    "viewer copy contract must name the pixel-world WASM payload",
)
require(
    "optional-payload-dir" in build_viewer,
    "viewer build wrapper must forward the optional payload staging directory",
)

if failures:
    raise AssertionError("\n".join(f"- {failure}" for failure in failures))
PY

echo "packaging-artifact-size-contract.test: OK"
