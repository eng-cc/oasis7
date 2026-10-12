#!/bin/bash
set -euo pipefail
app_root=$(cd "$(dirname "$0")/.." && pwd)
"$app_root/scripts/package-context-cli.sh"
cd "$app_root"
export OASIS7_SOURCE_HEAD_OID=$(git rev-parse HEAD)
export OASIS7_RELEASE_ID="context-$OASIS7_SOURCE_HEAD_OID"
export OASIS7_BUILD_TARGET=$(rustc -vV | sed -n 's/^host: //p')
exec tauri build --bundles app --config "$app_root/src-tauri/context-cli.bundle.json" "$@"
