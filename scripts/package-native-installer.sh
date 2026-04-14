#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLATFORM=""
BUNDLE_DIR=""
OUT_DIR=""
ASSET_NAME=""
VERSION=""
DRY_RUN=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/package-native-installer.sh [options]

Package a prepared launcher bundle into a platform-native release artifact.

Options:
  --platform <id>      required: linux-x64 | macos-x64 | windows-x64
  --bundle-dir <path>  required: prepared bundle directory to package
  --out-dir <path>     required: output directory for packaged installer
  --asset-name <name>  required: final asset filename (.deb | .dmg | .exe)
  --version <value>    required: release version (for example 0.0.40)
  --dry-run            print commands only; do not execute
  -h, --help           show this help
USAGE
}

run() {
  echo "+ $*"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi
  "$@"
}

ensure_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command not found: $cmd" >&2
    exit 1
  fi
}

resolve_abs_path() {
  local input="$1"
  if [[ "$input" == /* ]]; then
    printf '%s\n' "$input"
  else
    printf '%s\n' "$ROOT_DIR/$input"
  fi
}

normalize_release_version() {
  local value="${1#v}"
  value="$(printf '%s' "$value" | tr ' ' '-')"
  value="$(printf '%s' "$value" | sed -E 's/[^A-Za-z0-9.+:~_-]+/-/g')"
  value="$(printf '%s' "$value" | sed -E 's/-+/-/g; s/^-+//; s/-+$//')"
  printf '%s\n' "$value"
}

normalize_deb_version() {
  local value
  value="$(normalize_release_version "$1")"
  value="$(printf '%s' "$value" | sed -E 's/_/~/g')"
  printf '%s\n' "$value"
}

windows_native_path() {
  local path="$1"
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -aw "$path"
  else
    printf '%s\n' "$path"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --platform)
      PLATFORM="${2:-}"
      shift 2
      ;;
    --bundle-dir)
      BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --asset-name)
      ASSET_NAME="${2:-}"
      shift 2
      ;;
    --version)
      VERSION="${2:-}"
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

case "$PLATFORM" in
  linux-x64|macos-x64|windows-x64) ;;
  *)
    echo "error: --platform must be one of linux-x64|macos-x64|windows-x64" >&2
    exit 1
    ;;
esac

[[ -n "$BUNDLE_DIR" ]] || { echo "error: --bundle-dir is required" >&2; exit 1; }
[[ -n "$OUT_DIR" ]] || { echo "error: --out-dir is required" >&2; exit 1; }
[[ -n "$ASSET_NAME" ]] || { echo "error: --asset-name is required" >&2; exit 1; }
[[ -n "$VERSION" ]] || { echo "error: --version is required" >&2; exit 1; }

BUNDLE_DIR="$(resolve_abs_path "$BUNDLE_DIR")"
OUT_DIR="$(resolve_abs_path "$OUT_DIR")"
OUT_FILE="$OUT_DIR/$ASSET_NAME"
RELEASE_VERSION="$(normalize_release_version "$VERSION")"

if [[ -z "$RELEASE_VERSION" ]]; then
  echo "error: normalized release version is empty: $VERSION" >&2
  exit 1
fi

if [[ ! -d "$BUNDLE_DIR" && "$DRY_RUN" != "1" ]]; then
  echo "error: --bundle-dir does not exist: $BUNDLE_DIR" >&2
  exit 1
fi

run mkdir -p "$OUT_DIR"
run rm -f "$OUT_FILE"

case "$PLATFORM" in
  linux-x64)
    [[ "$ASSET_NAME" == *.deb ]] || { echo "error: linux-x64 asset must end with .deb" >&2; exit 1; }
    ensure_command dpkg-deb
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    PACKAGE_ROOT="$TMP_DIR/package"
    INSTALL_ROOT="$PACKAGE_ROOT/opt/oasis7"
    DEBIAN_DIR="$PACKAGE_ROOT/DEBIAN"
    BIN_DIR="$PACKAGE_ROOT/usr/bin"
    DEB_VERSION="$(normalize_deb_version "$VERSION")"

    run mkdir -p "$INSTALL_ROOT" "$DEBIAN_DIR" "$BIN_DIR"
    run cp -R "$BUNDLE_DIR/." "$INSTALL_ROOT/"

    if [[ "$DRY_RUN" == "1" ]]; then
      echo "+ write $DEBIAN_DIR/control"
      echo "+ write launcher symlinks under $BIN_DIR"
      echo "+ dpkg-deb --build --root-owner-group '$PACKAGE_ROOT' '$OUT_FILE'"
      exit 0
    fi

    cat > "$DEBIAN_DIR/control" <<EOF
Package: oasis7
Version: $DEB_VERSION
Section: games
Priority: optional
Architecture: amd64
Maintainer: oasis7 release automation <noreply@oasis7.invalid>
Description: oasis7 technical preview bundle
 A platform preview bundle for oasis7 release validation and local technical
 verification.
EOF

    ln -s /opt/oasis7/run-client.sh "$BIN_DIR/oasis7-client"
    ln -s /opt/oasis7/run-game.sh "$BIN_DIR/oasis7-game"
    ln -s /opt/oasis7/run-web-launcher.sh "$BIN_DIR/oasis7-web-launcher"
    ln -s /opt/oasis7/run-chain-runtime.sh "$BIN_DIR/oasis7-chain-runtime"

    dpkg-deb --build --root-owner-group "$PACKAGE_ROOT" "$OUT_FILE"
    ;;
  macos-x64)
    [[ "$ASSET_NAME" == *.dmg ]] || { echo "error: macos-x64 asset must end with .dmg" >&2; exit 1; }
    ensure_command hdiutil
    run hdiutil create -volname "oasis7 $RELEASE_VERSION" -srcfolder "$BUNDLE_DIR" -ov -format UDZO "$OUT_FILE"
    ;;
  windows-x64)
    [[ "$ASSET_NAME" == *.exe ]] || { echo "error: windows-x64 asset must end with .exe" >&2; exit 1; }
    ensure_command pwsh
    BUNDLE_DIR_NATIVE="$(windows_native_path "$BUNDLE_DIR")"
    OUT_FILE_NATIVE="$(windows_native_path "$OUT_FILE")"
    if [[ "$DRY_RUN" == "1" ]]; then
      echo "+ pwsh -NoLogo -NoProfile -Command <7z sfx packaging> '$BUNDLE_DIR_NATIVE' -> '$OUT_FILE_NATIVE'"
      exit 0
    fi

    export OASIS7_WINDOWS_BUNDLE_DIR="$BUNDLE_DIR_NATIVE"
    export OASIS7_WINDOWS_OUT_FILE="$OUT_FILE_NATIVE"
    pwsh -NoLogo -NoProfile -Command '
      $ErrorActionPreference = "Stop"

      function Find-SevenZipSfxModule {
        param(
          [string]$SevenZipExecutable
        )

        $candidateDirs = New-Object System.Collections.Generic.List[string]
        if ($SevenZipExecutable) {
          $candidateDirs.Add((Split-Path -Parent $SevenZipExecutable))
        }

        foreach ($programFilesDir in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
          if ($programFilesDir) {
            $candidateDirs.Add((Join-Path $programFilesDir "7-Zip"))
          }
        }

        if ($env:ChocolateyInstall) {
          $candidateDirs.Add((Join-Path $env:ChocolateyInstall "bin"))
          $candidateDirs.Add((Join-Path $env:ChocolateyInstall "lib\\7zip\\tools"))
        }

        $seen = @{}
        foreach ($dir in $candidateDirs) {
          if (-not $dir -or $seen.ContainsKey($dir)) {
            continue
          }
          $seen[$dir] = $true
          foreach ($moduleName in @("7z.sfx", "7zCon.sfx")) {
            $modulePath = Join-Path $dir $moduleName
            if (Test-Path $modulePath) {
              return $modulePath
            }
          }
        }

        $searched = ($candidateDirs | Where-Object { $_ } | Select-Object -Unique) -join ", "
        throw "7z SFX module not found. Searched: $searched"
      }

      $bundleDir = $env:OASIS7_WINDOWS_BUNDLE_DIR
      $outFile = $env:OASIS7_WINDOWS_OUT_FILE
      $sevenZip = (Get-Command 7z -ErrorAction Stop).Source
      $sfxModule = Find-SevenZipSfxModule -SevenZipExecutable $sevenZip

      $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("oasis7-sfx-" + [Guid]::NewGuid().ToString("N"))
      New-Item -ItemType Directory -Path $tempRoot | Out-Null
      try {
        $archivePath = Join-Path $tempRoot "payload.7z"
        & $sevenZip a -t7z -mx=9 $archivePath (Join-Path $bundleDir "*") | Out-Host
        if ($LASTEXITCODE -ne 0) {
          throw "7z archive creation failed with exit code $LASTEXITCODE"
        }

        $parentDir = Split-Path -Parent $outFile
        if (-not (Test-Path $parentDir)) {
          New-Item -ItemType Directory -Path $parentDir | Out-Null
        }
        if (Test-Path $outFile) {
          Remove-Item $outFile -Force
        }

        $outputStream = [System.IO.File]::Open($outFile, [System.IO.FileMode]::CreateNew)
        try {
          foreach ($part in @($sfxModule, $archivePath)) {
            $bytes = [System.IO.File]::ReadAllBytes($part)
            $outputStream.Write($bytes, 0, $bytes.Length)
          }
        }
        finally {
          $outputStream.Dispose()
        }
      }
      finally {
        if (Test-Path $tempRoot) {
          Remove-Item $tempRoot -Recurse -Force
        }
      }
    '
    ;;
esac

echo "Packaged installer asset: $OUT_FILE"
