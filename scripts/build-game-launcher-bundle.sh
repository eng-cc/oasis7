#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/bundle-freshness-lib.sh"
OUT_DIR=""
PROFILE="release"
TARGET_TRIPLE="native"
WEB_DIST_SOURCE=""
WEB_LAUNCHER_DIST_SOURCE=""
DRY_RUN=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/build-game-launcher-bundle.sh [options]

Build a distributable launcher bundle:
- bin/oasis7_client_launcher
- bin/oasis7_game_launcher
- bin/oasis7_web_launcher
- bin/oasis7_viewer_live
- bin/oasis7_chain_runtime
- web/ (prebuilt viewer static assets)
- web-launcher/ (prebuilt launcher web static assets)
- run-client.sh (desktop client launcher entry)
- run-web-launcher.sh (headless web console launcher entry)
- run-game.sh (one-command entry)
- run-chain-runtime.sh (direct chain runtime entry)

Options:
  --out-dir <path>       output directory (default: output/release/game-launcher-<timestamp>)
  --profile <name>       cargo profile: release|dev (default: release)
  --target-triple <id>   rust target triple (default: native)
  --web-dist <path>      use existing prebuilt viewer web dist instead of trunk build
  --web-launcher-dist <path>
                         use existing prebuilt launcher web dist instead of trunk build
  --dry-run              print commands only; do not execute
  -h, --help             show this help
USAGE
}

run() {
  echo "+ $*"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi
  "$@"
}

replace_file() {
  local src="$1"
  local dest="$2"
  # Remove destination first so running binaries don't trigger ETXTBSY on overwrite.
  run rm -f "$dest"
  run cp "$src" "$dest"
}

ensure_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command not found: $cmd" >&2
    exit 1
  fi
}

active_rust_toolchain() {
  rustup show active-toolchain 2>/dev/null | awk 'NR == 1 { print $1 }'
}

ensure_rust_target_installed() {
  local target="$1"
  local toolchain="${2:-}"
  local add_args=(target add "$target")
  if [[ -n "$toolchain" ]]; then
    add_args+=(--toolchain "$toolchain")
  fi
  run rustup "${add_args[@]}"
}

validate_prebuilt_dist_has_index() {
  local option_name="$1"
  local web_dist="$2"
  local index_html="$web_dist/index.html"
  if [[ ! -f "$index_html" ]]; then
    echo "error: ${option_name} must contain index.html: $web_dist" >&2
    exit 1
  fi
}

validate_web_dist_source() {
  local web_dist="$1"
  local index_html="$web_dist/index.html"
  validate_prebuilt_dist_has_index "--web-dist" "$web_dist"

  # Guardrail: this script often gets pointed at top-level `site/`, which is
  # docs/marketing pages and will open GitHub Pages instead of the game viewer.
  if grep -E -q "eng-cc\.github\.io/oasis7|doc/cn/index.html|会进化的文明战争游戏" "$index_html"; then
    echo "error: --web-dist appears to be docs/marketing site, not viewer web dist: $web_dist" >&2
    echo "hint: remove --web-dist to let script run trunk build automatically," >&2
    echo "      or pass a dist directory built from crates/oasis7_viewer." >&2
    exit 1
  fi
}

validate_web_launcher_dist_source() {
  validate_prebuilt_dist_has_index "--web-launcher-dist" "$1"
}

resolve_binary_name() {
  local base="$1"
  local target_triple="$2"
  if [[ "$target_triple" == *windows* ]]; then
    echo "${base}.exe"
  elif [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]]; then
    echo "${base}.exe"
  else
    echo "$base"
  fi
}

bundle_platform_id() {
  local target_triple="$1"
  if [[ "$target_triple" == "native" ]]; then
    if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]]; then
      echo "windows-x64"
    elif [[ "$(uname -s)" == "Darwin" ]]; then
      echo "macos-x64"
    else
      echo "linux-x64"
    fi
  elif [[ "$target_triple" == *windows* ]]; then
    echo "windows-x64"
  elif [[ "$target_triple" == *apple-darwin* || "$target_triple" == *darwin* ]]; then
    echo "macos-x64"
  else
    echo "linux-x64"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --profile)
      PROFILE="${2:-}"
      shift 2
      ;;
    --target-triple)
      TARGET_TRIPLE="${2:-}"
      shift 2
      ;;
    --web-dist)
      WEB_DIST_SOURCE="${2:-}"
      shift 2
      ;;
    --web-launcher-dist)
      WEB_LAUNCHER_DIST_SOURCE="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$OUT_DIR" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  OUT_DIR="$ROOT_DIR/output/release/game-launcher-$ts"
fi
if [[ "$OUT_DIR" != /* ]]; then
  OUT_DIR="$ROOT_DIR/$OUT_DIR"
fi
if [[ -n "$WEB_DIST_SOURCE" && "$WEB_DIST_SOURCE" != /* ]]; then
  WEB_DIST_SOURCE="$ROOT_DIR/$WEB_DIST_SOURCE"
fi
if [[ -n "$WEB_LAUNCHER_DIST_SOURCE" && "$WEB_LAUNCHER_DIST_SOURCE" != /* ]]; then
  WEB_LAUNCHER_DIST_SOURCE="$ROOT_DIR/$WEB_LAUNCHER_DIST_SOURCE"
fi

if [[ "$PROFILE" != "release" && "$PROFILE" != "dev" ]]; then
  echo "error: --profile must be release or dev" >&2
  exit 1
fi
if [[ -z "$TARGET_TRIPLE" ]]; then
  echo "error: --target-triple must not be empty" >&2
  exit 1
fi

if [[ -n "$WEB_DIST_SOURCE" && ! -d "$WEB_DIST_SOURCE" ]]; then
  echo "error: --web-dist path does not exist: $WEB_DIST_SOURCE" >&2
  exit 1
fi
if [[ -n "$WEB_DIST_SOURCE" ]]; then
  validate_web_dist_source "$WEB_DIST_SOURCE"
fi
if [[ -n "$WEB_LAUNCHER_DIST_SOURCE" && ! -d "$WEB_LAUNCHER_DIST_SOURCE" ]]; then
  echo "error: --web-launcher-dist path does not exist: $WEB_LAUNCHER_DIST_SOURCE" >&2
  exit 1
fi
if [[ -n "$WEB_LAUNCHER_DIST_SOURCE" ]]; then
  validate_web_launcher_dist_source "$WEB_LAUNCHER_DIST_SOURCE"
fi

LAUNCHER_BIN_NAME="$(resolve_binary_name oasis7_game_launcher "$TARGET_TRIPLE")"
WEB_LAUNCHER_BIN_NAME="$(resolve_binary_name oasis7_web_launcher "$TARGET_TRIPLE")"
LIVE_BIN_NAME="$(resolve_binary_name oasis7_viewer_live "$TARGET_TRIPLE")"
CHAIN_BIN_NAME="$(resolve_binary_name oasis7_chain_runtime "$TARGET_TRIPLE")"
CLIENT_LAUNCHER_BIN_NAME="$(resolve_binary_name oasis7_client_launcher "$TARGET_TRIPLE")"
TARGET_SUBDIR="$PROFILE"
if [[ "$PROFILE" == "dev" ]]; then
  TARGET_SUBDIR="debug"
fi
TARGET_OUTPUT_SUBDIR="$TARGET_SUBDIR"
CARGO_TARGET_ARGS=()
if [[ "$TARGET_TRIPLE" != "native" ]]; then
  TARGET_OUTPUT_SUBDIR="$TARGET_TRIPLE/$TARGET_SUBDIR"
  CARGO_TARGET_ARGS=(--target "$TARGET_TRIPLE")
fi
BUNDLE_PLATFORM_ID="$(bundle_platform_id "$TARGET_TRIPLE")"

BUNDLE_BIN_DIR="$OUT_DIR/bin"
BUNDLE_WEB_DIR="$OUT_DIR/web"
BUNDLE_WEB_LAUNCHER_DIR="$OUT_DIR/web-launcher"

run mkdir -p "$BUNDLE_BIN_DIR" "$BUNDLE_WEB_DIR" "$BUNDLE_WEB_LAUNCHER_DIR"

# 1) Build native binaries for launcher/live/client launcher.
BUNDLE_NATIVE_BUILD_ARGS=(
  "${CARGO_TARGET_ARGS[@]}"
  -p oasis7
  -p oasis7_client_launcher
  --bin oasis7_game_launcher
  --bin oasis7_web_launcher
  --bin oasis7_viewer_live
  --bin oasis7_chain_runtime
  --bin oasis7_client_launcher
)
if [[ "$PROFILE" == "release" ]]; then
  run env -u RUSTC_WRAPPER cargo build --release "${BUNDLE_NATIVE_BUILD_ARGS[@]}"
else
  run env -u RUSTC_WRAPPER cargo build "${BUNDLE_NATIVE_BUILD_ARGS[@]}"
fi

LAUNCHER_SRC="$ROOT_DIR/target/$TARGET_OUTPUT_SUBDIR/$LAUNCHER_BIN_NAME"
WEB_LAUNCHER_SRC="$ROOT_DIR/target/$TARGET_OUTPUT_SUBDIR/$WEB_LAUNCHER_BIN_NAME"
LIVE_SRC="$ROOT_DIR/target/$TARGET_OUTPUT_SUBDIR/$LIVE_BIN_NAME"
CHAIN_SRC="$ROOT_DIR/target/$TARGET_OUTPUT_SUBDIR/$CHAIN_BIN_NAME"
CLIENT_LAUNCHER_SRC="$ROOT_DIR/target/$TARGET_OUTPUT_SUBDIR/$CLIENT_LAUNCHER_BIN_NAME"

if [[ "$DRY_RUN" != "1" ]]; then
  [[ -f "$LAUNCHER_SRC" ]] || { echo "error: launcher binary not found: $LAUNCHER_SRC" >&2; exit 1; }
  [[ -f "$WEB_LAUNCHER_SRC" ]] || { echo "error: web launcher binary not found: $WEB_LAUNCHER_SRC" >&2; exit 1; }
  [[ -f "$LIVE_SRC" ]] || { echo "error: oasis7_viewer_live binary not found: $LIVE_SRC" >&2; exit 1; }
  [[ -f "$CHAIN_SRC" ]] || { echo "error: oasis7_chain_runtime binary not found: $CHAIN_SRC" >&2; exit 1; }
  [[ -f "$CLIENT_LAUNCHER_SRC" ]] || { echo "error: client launcher binary not found: $CLIENT_LAUNCHER_SRC" >&2; exit 1; }
fi

replace_file "$LAUNCHER_SRC" "$BUNDLE_BIN_DIR/$LAUNCHER_BIN_NAME"
replace_file "$WEB_LAUNCHER_SRC" "$BUNDLE_BIN_DIR/$WEB_LAUNCHER_BIN_NAME"
replace_file "$LIVE_SRC" "$BUNDLE_BIN_DIR/$LIVE_BIN_NAME"
replace_file "$CHAIN_SRC" "$BUNDLE_BIN_DIR/$CHAIN_BIN_NAME"
replace_file "$CLIENT_LAUNCHER_SRC" "$BUNDLE_BIN_DIR/$CLIENT_LAUNCHER_BIN_NAME"

# 2) Prepare viewer web dist (software_safe static bundle by default).
if [[ -n "$WEB_DIST_SOURCE" ]]; then
  run rm -rf "$BUNDLE_WEB_DIR"
  run mkdir -p "$BUNDLE_WEB_DIR"
  run cp -R "$WEB_DIST_SOURCE/." "$BUNDLE_WEB_DIR/"
else
  ensure_command npm
  run rm -rf "$BUNDLE_WEB_DIR"
  run mkdir -p "$BUNDLE_WEB_DIR"
  run bash -lc "cd '$ROOT_DIR' && npm --prefix crates/oasis7_viewer run build:software-safe"
  run cp "$ROOT_DIR/crates/oasis7_viewer/software_safe.html" "$BUNDLE_WEB_DIR/index.html"
  run cp "$ROOT_DIR/crates/oasis7_viewer/software_safe.html" "$BUNDLE_WEB_DIR/software_safe.html"
  run cp "$ROOT_DIR/crates/oasis7_viewer/software_safe.js" "$BUNDLE_WEB_DIR/software_safe.js"
  run cp "$ROOT_DIR/crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html" "$BUNDLE_WEB_DIR/software_safe_first_agent_claim_evidence.html"
  run cp "$ROOT_DIR/crates/oasis7_viewer/favicon.ico" "$BUNDLE_WEB_DIR/favicon.ico"
fi

# 3) Prepare launcher web dist (prebuilt artifact preferred; trunk build fallback).
if [[ -n "$WEB_LAUNCHER_DIST_SOURCE" ]]; then
  run rm -rf "$BUNDLE_WEB_LAUNCHER_DIR"
  run mkdir -p "$BUNDLE_WEB_LAUNCHER_DIR"
  run cp -R "$WEB_LAUNCHER_DIST_SOURCE/." "$BUNDLE_WEB_LAUNCHER_DIR/"
else
  ensure_command trunk
  ensure_command rustup
  ACTIVE_RUST_TOOLCHAIN="$(active_rust_toolchain)"
  ensure_rust_target_installed "wasm32-unknown-unknown" "$ACTIVE_RUST_TOOLCHAIN"

  run rm -rf "$BUNDLE_WEB_LAUNCHER_DIR"
  run mkdir -p "$BUNDLE_WEB_LAUNCHER_DIR"
  if [[ "$PROFILE" == "release" ]]; then
    run bash -lc "cd '$ROOT_DIR/crates/oasis7_client_launcher' && env -u NO_COLOR trunk build --release --dist '$BUNDLE_WEB_LAUNCHER_DIR'"
  else
    run bash -lc "cd '$ROOT_DIR/crates/oasis7_client_launcher' && env -u NO_COLOR trunk build --dist '$BUNDLE_WEB_LAUNCHER_DIR'"
  fi
fi

bundle_write_manifest "$ROOT_DIR" "$OUT_DIR"

# 4) Generate desktop client wrapper + one-command CLI wrapper and readme.
run bash -lc "cat > '$OUT_DIR/run-client.sh' <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR=\"\$(cd \"\$(dirname \"\${BASH_SOURCE[0]}\")\" && pwd)\"
OASIS7_GAME_LAUNCHER_BIN=\"\$ROOT_DIR/bin/$LAUNCHER_BIN_NAME\" \
OASIS7_GAME_STATIC_DIR=\"\$ROOT_DIR/web\" \
OASIS7_CHAIN_RUNTIME_BIN=\"\$ROOT_DIR/bin/$CHAIN_BIN_NAME\" \
\"\$ROOT_DIR/bin/$CLIENT_LAUNCHER_BIN_NAME\" \"\$@\"
LAUNCH"
run chmod +x "$OUT_DIR/run-client.sh"

run bash -lc "cat > '$OUT_DIR/run-web-launcher.sh' <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR=\"\$(cd \"\$(dirname \"\${BASH_SOURCE[0]}\")\" && pwd)\"
CHAIN_STORAGE_PROFILE=\"\${OASIS7_CHAIN_STORAGE_PROFILE:-}\"
CHAIN_STORAGE_PROFILE_ARGS=()
if [[ -n \"\$CHAIN_STORAGE_PROFILE\" ]]; then
  CHAIN_STORAGE_PROFILE_ARGS=(--chain-storage-profile \"\$CHAIN_STORAGE_PROFILE\")
fi
OASIS7_GAME_LAUNCHER_BIN=\"\$ROOT_DIR/bin/$LAUNCHER_BIN_NAME\" \
OASIS7_GAME_STATIC_DIR=\"\$ROOT_DIR/web\" \
OASIS7_CHAIN_RUNTIME_BIN=\"\$ROOT_DIR/bin/$CHAIN_BIN_NAME\" \
OASIS7_WEB_LAUNCHER_STATIC_DIR=\"\$ROOT_DIR/web-launcher\" \
\"\$ROOT_DIR/bin/$WEB_LAUNCHER_BIN_NAME\" \"\${CHAIN_STORAGE_PROFILE_ARGS[@]}\" \"\$@\"
LAUNCH"
run chmod +x "$OUT_DIR/run-web-launcher.sh"

run bash -lc "cat > '$OUT_DIR/run-chain-runtime.sh' <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR=\"\$(cd \"\$(dirname \"\${BASH_SOURCE[0]}\")\" && pwd)\"
CHAIN_STORAGE_PROFILE=\"\${OASIS7_CHAIN_STORAGE_PROFILE:-}\"
CHAIN_STORAGE_PROFILE_ARGS=()
if [[ -n \"\$CHAIN_STORAGE_PROFILE\" ]]; then
  CHAIN_STORAGE_PROFILE_ARGS=(--storage-profile \"\$CHAIN_STORAGE_PROFILE\")
fi
\"\$ROOT_DIR/bin/$CHAIN_BIN_NAME\" \"\${CHAIN_STORAGE_PROFILE_ARGS[@]}\" \"\$@\"
LAUNCH"
run chmod +x "$OUT_DIR/run-chain-runtime.sh"

run bash -lc "cat > '$OUT_DIR/run-game.sh' <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR=\"\$(cd \"\$(dirname \"\${BASH_SOURCE[0]}\")\" && pwd)\"
CHAIN_STORAGE_PROFILE=\"\${OASIS7_CHAIN_STORAGE_PROFILE:-}\"
CHAIN_STORAGE_PROFILE_ARGS=()
if [[ -n \"\$CHAIN_STORAGE_PROFILE\" ]]; then
  CHAIN_STORAGE_PROFILE_ARGS=(--chain-storage-profile \"\$CHAIN_STORAGE_PROFILE\")
fi
OASIS7_CHAIN_RUNTIME_BIN=\"\$ROOT_DIR/bin/$CHAIN_BIN_NAME\" \
\"\$ROOT_DIR/bin/$LAUNCHER_BIN_NAME\" --viewer-static-dir \"\$ROOT_DIR/web\" \"\${CHAIN_STORAGE_PROFILE_ARGS[@]}\" \"\$@\"
LAUNCH"
run chmod +x "$OUT_DIR/run-game.sh"

if [[ "$BUNDLE_PLATFORM_ID" == "windows-x64" ]]; then
  run bash -lc "cat > '$OUT_DIR/run-client.cmd' <<'LAUNCH'
@echo off
setlocal
set \"ROOT_DIR=%~dp0\"
set \"OASIS7_GAME_LAUNCHER_BIN=%ROOT_DIR%bin\\$LAUNCHER_BIN_NAME\"
set \"OASIS7_GAME_STATIC_DIR=%ROOT_DIR%web\"
set \"OASIS7_CHAIN_RUNTIME_BIN=%ROOT_DIR%bin\\$CHAIN_BIN_NAME\"
\"%ROOT_DIR%bin\\$CLIENT_LAUNCHER_BIN_NAME\" %*
LAUNCH"

  run bash -lc "cat > '$OUT_DIR/run-web-launcher.cmd' <<'LAUNCH'
@echo off
setlocal
set \"ROOT_DIR=%~dp0\"
set \"OASIS7_GAME_LAUNCHER_BIN=%ROOT_DIR%bin\\$LAUNCHER_BIN_NAME\"
set \"OASIS7_GAME_STATIC_DIR=%ROOT_DIR%web\"
set \"OASIS7_CHAIN_RUNTIME_BIN=%ROOT_DIR%bin\\$CHAIN_BIN_NAME\"
set \"OASIS7_WEB_LAUNCHER_STATIC_DIR=%ROOT_DIR%web-launcher\"
if defined OASIS7_CHAIN_STORAGE_PROFILE (
  \"%ROOT_DIR%bin\\$WEB_LAUNCHER_BIN_NAME\" --chain-storage-profile \"%OASIS7_CHAIN_STORAGE_PROFILE%\" %*
) else (
  \"%ROOT_DIR%bin\\$WEB_LAUNCHER_BIN_NAME\" %*
)
LAUNCH"

  run bash -lc "cat > '$OUT_DIR/run-chain-runtime.cmd' <<'LAUNCH'
@echo off
setlocal
set \"ROOT_DIR=%~dp0\"
if defined OASIS7_CHAIN_STORAGE_PROFILE (
  \"%ROOT_DIR%bin\\$CHAIN_BIN_NAME\" --storage-profile \"%OASIS7_CHAIN_STORAGE_PROFILE%\" %*
) else (
  \"%ROOT_DIR%bin\\$CHAIN_BIN_NAME\" %*
)
LAUNCH"

  run bash -lc "cat > '$OUT_DIR/run-game.cmd' <<'LAUNCH'
@echo off
setlocal
set \"ROOT_DIR=%~dp0\"
set \"OASIS7_CHAIN_RUNTIME_BIN=%ROOT_DIR%bin\\$CHAIN_BIN_NAME\"
set \"OASIS7_GAME_STATIC_DIR=%ROOT_DIR%web\"
if defined OASIS7_CHAIN_STORAGE_PROFILE (
  \"%ROOT_DIR%bin\\$LAUNCHER_BIN_NAME\" --chain-storage-profile \"%OASIS7_CHAIN_STORAGE_PROFILE%\" %*
) else (
  \"%ROOT_DIR%bin\\$LAUNCHER_BIN_NAME\" %*
)
LAUNCH"
fi

if [[ "$BUNDLE_PLATFORM_ID" == "macos-x64" ]]; then
  BUNDLE_MACOS_APP_DIR="$OUT_DIR/oasis7 Client Launcher.app"
  run mkdir -p "$BUNDLE_MACOS_APP_DIR/Contents/MacOS" "$BUNDLE_MACOS_APP_DIR/Contents/Resources"
  run bash -lc "cat > '$BUNDLE_MACOS_APP_DIR/Contents/Info.plist' <<'PLIST'
<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>oasis7-client-launcher</string>
  <key>CFBundleIdentifier</key>
  <string>com.oasis7.client-launcher</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>oasis7 Client Launcher</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
</dict>
</plist>
PLIST"
  run bash -lc "cat > '$BUNDLE_MACOS_APP_DIR/Contents/MacOS/oasis7-client-launcher' <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
APP_ROOT=\"\$(cd \"\$(dirname \"\${BASH_SOURCE[0]}\")/../../..\" && pwd)\"
exec \"\$APP_ROOT/run-client.sh\" \"\$@\"
LAUNCH"
  run chmod +x "$BUNDLE_MACOS_APP_DIR/Contents/MacOS/oasis7-client-launcher"
fi

run bash -lc "cat > '$OUT_DIR/README.txt' <<'README'
oasis7 Launcher Bundle

Quick start:
1) Desktop launcher: ./run-client.sh
2) Web launcher (headless): ./run-web-launcher.sh --listen-bind 0.0.0.0:5410
3) CLI launcher: ./run-game.sh
4) Direct chain runtime: ./run-chain-runtime.sh
5) Open URL printed by launcher (CLI path defaults auto-open browser).
6) macOS DMG: open oasis7 Client Launcher.app when present.
7) Windows bundle: use run-client.cmd / run-game.cmd when present.

Optional:
- Desktop launcher can start/stop game stack from GUI and open URL in one click.
- Web launcher provides a browser control panel for headless server operation.
- Enable LLM mode: ./run-game.sh --with-llm
- Disable auto-open browser: ./run-game.sh --no-open-browser
- Override chain storage profile without hardcoding wrapper defaults:
  OASIS7_CHAIN_STORAGE_PROFILE=release_default ./run-game.sh
  OASIS7_CHAIN_STORAGE_PROFILE=soak_forensics ./run-web-launcher.sh --listen-bind 0.0.0.0:5410

Upgrade policy (current truth):
- Re-download the latest primary package and manually replace/overwrite the current install.
- There is no in-app updater or automatic config/world migration in this bundle yet.
- Back up these paths from the directory you actually launch from before replacing the bundle:
  - config.toml
  - .oasis7_launcher_ux_state.json
  - output/chain-runtime/<node_id>/reward-runtime-execution-world/
- On Windows, do not assume uninstall + reinstall preserves local state. The current uninstaller removes the install directory.

Bundle layout:
- bin/oasis7_client_launcher
- bin/oasis7_game_launcher
- bin/oasis7_web_launcher
- bin/oasis7_viewer_live
- bin/oasis7_chain_runtime
- web/
- web-launcher/
- .oasis7-bundle-manifest.json
- run-client.sh
- run-web-launcher.sh
- run-game.sh
- run-chain-runtime.sh
- run-client.cmd (Windows bundle only)
- run-web-launcher.cmd (Windows bundle only)
- run-game.cmd (Windows bundle only)
- run-chain-runtime.cmd (Windows bundle only)
- oasis7 Client Launcher.app (macOS bundle only)
README"

if [[ "$DRY_RUN" != "1" ]]; then
  "$ROOT_DIR/scripts/validate-release-platform-entrypoints.sh" \
    --platform "$BUNDLE_PLATFORM_ID" \
    --bundle-dir "$OUT_DIR"
fi

cat <<INFO
Bundle ready: $OUT_DIR
- client launcher: $BUNDLE_BIN_DIR/$CLIENT_LAUNCHER_BIN_NAME
- launcher:        $BUNDLE_BIN_DIR/$LAUNCHER_BIN_NAME
- web launcher:    $BUNDLE_BIN_DIR/$WEB_LAUNCHER_BIN_NAME
- live:            $BUNDLE_BIN_DIR/$LIVE_BIN_NAME
- chain runtime:   $BUNDLE_BIN_DIR/$CHAIN_BIN_NAME
- web:             $BUNDLE_WEB_DIR
- web launcher:    $BUNDLE_WEB_LAUNCHER_DIR
- entries:         $OUT_DIR/run-client.sh, $OUT_DIR/run-web-launcher.sh, $OUT_DIR/run-game.sh, $OUT_DIR/run-chain-runtime.sh
- platform:        $BUNDLE_PLATFORM_ID
INFO
