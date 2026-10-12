#!/bin/bash
set -euo pipefail
app_root=$(cd "$(dirname "$0")/.." && pwd)
requested_target=""
awaiting_target=false
runner_args=false
for argument in "$@"; do
  value=""
  if [[ "$runner_args" == true ]]; then
    case "$argument" in --target|--target=*|-t|-t*) echo "Set the shared target before --; runner target overrides are unsupported" >&2; exit 1 ;; esac
    continue
  fi
  if [[ "$awaiting_target" == true ]]; then
    value="$argument"
    awaiting_target=false
  else
    case "$argument" in
      --) runner_args=true; continue ;;
      --target|-t) awaiting_target=true; continue ;;
      --target=*) value="${argument#--target=}" ;;
      -t=*) value="${argument#-t=}" ;;
      -t*) value="${argument#-t}" ;;
      *) continue ;;
    esac
  fi
  if [[ -z "$value" || -n "$requested_target" ]]; then
    echo "A single nonempty build target is required" >&2; exit 1
  fi
  requested_target="$value"
done
if [[ "$awaiting_target" == true ]]; then echo "--target requires a Rust target triple" >&2; exit 1; fi
build_target=${requested_target:-$(rustc -vV | sed -n 's/^host: //p')}
if [[ "$build_target" == universal-apple-darwin ]]; then
  echo "Universal App/CLI packaging is unsupported; build a single Apple target" >&2; exit 1
fi
"$app_root/scripts/package-context-cli.sh" --target "$build_target"
cd "$app_root"
export OASIS7_SOURCE_HEAD_OID=$(git rev-parse HEAD)
export OASIS7_RELEASE_ID="context-$OASIS7_SOURCE_HEAD_OID"
export OASIS7_BUILD_TARGET="$build_target"
if [[ -z "$requested_target" ]]; then set -- --target "$build_target" "$@"; fi
exec tauri build --bundles app --config "$app_root/src-tauri/context-cli.bundle.json" "$@"
