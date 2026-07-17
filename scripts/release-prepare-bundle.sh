#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLATFORM=""
TARGET_TRIPLE="native"
WEB_DIST=""
WEB_LAUNCHER_DIST=""
OUT_DIR=""
PROFILE="release"

usage() {
  cat <<'USAGE'
Usage: ./scripts/release-prepare-bundle.sh [options]

Prepare deterministic launcher bundle directory for CI release packaging.

Options:
  --platform <id>        required: linux-x64 | macos-x64 | macos-arm64 | windows-x64
  --target-triple <id>   optional rust target triple (default: native)
  --web-dist <path>      required: prebuilt viewer web dist directory
  --web-launcher-dist <path>
                         required: prebuilt launcher web dist directory
  --out-dir <path>       required: output root for prepared bundle directory
  --profile <name>       cargo profile: release|dev (default: release)
  -h, --help             show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --platform)
      PLATFORM="${2:-}"
      shift 2
      ;;
    --target-triple)
      TARGET_TRIPLE="${2:-}"
      shift 2
      ;;
    --web-dist)
      WEB_DIST="${2:-}"
      shift 2
      ;;
    --web-launcher-dist)
      WEB_LAUNCHER_DIST="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --profile)
      PROFILE="${2:-}"
      shift 2
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

case "${PLATFORM}" in
  linux-x64|macos-x64|macos-arm64|windows-x64) ;;
  *)
    echo "error: --platform must be one of linux-x64|macos-x64|macos-arm64|windows-x64" >&2
    exit 1
    ;;
esac

if [[ -z "${WEB_DIST}" ]]; then
  echo "error: --web-dist is required" >&2
  exit 1
fi
if [[ -z "${WEB_LAUNCHER_DIST}" ]]; then
  echo "error: --web-launcher-dist is required" >&2
  exit 1
fi
if [[ -z "${OUT_DIR}" ]]; then
  echo "error: --out-dir is required" >&2
  exit 1
fi
if [[ -z "${TARGET_TRIPLE}" ]]; then
  echo "error: --target-triple must not be empty" >&2
  exit 1
fi
if [[ "${PROFILE}" != "release" && "${PROFILE}" != "dev" ]]; then
  echo "error: --profile must be release or dev" >&2
  exit 1
fi

if [[ "${WEB_DIST}" != /* ]]; then
  WEB_DIST="${ROOT_DIR}/${WEB_DIST}"
fi
if [[ "${WEB_LAUNCHER_DIST}" != /* ]]; then
  WEB_LAUNCHER_DIST="${ROOT_DIR}/${WEB_LAUNCHER_DIST}"
fi
if [[ "${OUT_DIR}" != /* ]]; then
  OUT_DIR="${ROOT_DIR}/${OUT_DIR}"
fi

if [[ ! -d "${WEB_DIST}" ]]; then
  echo "error: web dist path does not exist: ${WEB_DIST}" >&2
  exit 1
fi
if [[ ! -d "${WEB_LAUNCHER_DIST}" ]]; then
  echo "error: web launcher dist path does not exist: ${WEB_LAUNCHER_DIST}" >&2
  exit 1
fi

BUNDLE_DIR="${OUT_DIR}/oasis7-${PLATFORM}"
rm -rf "${BUNDLE_DIR}"
mkdir -p "${OUT_DIR}"

"${ROOT_DIR}/scripts/build-game-launcher-bundle.sh" \
  --out-dir "${BUNDLE_DIR}" \
  --profile "${PROFILE}" \
  --target-triple "${TARGET_TRIPLE}" \
  --web-dist "${WEB_DIST}" \
  --web-launcher-dist "${WEB_LAUNCHER_DIST}"

echo "Prepared bundle directory: ${BUNDLE_DIR}"
