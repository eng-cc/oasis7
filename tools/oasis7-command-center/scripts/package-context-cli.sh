#!/bin/bash
set -euo pipefail
# Build from the same checkout as the App. No network download or PATH lookup at runtime.
repo_root=$(git rev-parse --show-toplevel)
app_root="$repo_root/tools/oasis7-command-center"
if ! git -C "$repo_root" diff --quiet || ! git -C "$repo_root" diff --cached --quiet; then
  echo "Package from a committed source tree so the CLI catalog identifies the actual build." >&2
  exit 1
fi
head=$(git -C "$repo_root" rev-parse HEAD)
target=$(rustc -vV | sed -n 's/^host: //p')
release="context-$head"
OASIS7_SOURCE_HEAD_OID="$head" OASIS7_RELEASE_ID="$release" OASIS7_BUILD_TARGET="$target" \
  "$repo_root/scripts/cargo-dev.sh" build --locked --release -p oasis7_context
binary="$repo_root/target/release/oasis7_context"
out="$app_root/src-tauri/resources/context-cli"
mkdir -p "$out"
staging=$(mktemp "$out/.context-cli.XXXXXX")
trap 'rm -f "$staging"' EXIT
cp "$binary" "$staging"
chmod 555 "$staging"
mv -f "$staging" "$out/oasis7_context"
python3 - "$out" "$head" "$target" "$release" <<'PYTHON'
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1]); data=(root/'oasis7_context').read_bytes()
(root/'catalog.json').write_text(json.dumps(dict(schema_version=1,tool_id='oasis7_context',source_head_oid=sys.argv[2],target=sys.argv[3],release_id=sys.argv[4],file='oasis7_context',sha256=hashlib.sha256(data).hexdigest(),size_bytes=len(data)),sort_keys=True,separators=(',',':'))+'\n')
PYTHON
