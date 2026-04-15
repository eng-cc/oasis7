#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

SITE_ENTRIES=(
  "${REPO_ROOT}/site/index.html"
  "${REPO_ROOT}/site/en/index.html"
)

REQUIRED_ENTRY_MARKERS=(
  "data-release-tag"
  "data-release-date"
  "data-release-notes-link"
  "data-download-surface"
  "data-download-primary-link"
  "data-download-primary-requirements"
  "data-download-primary-install"
  "data-download-primary-trust"
  "data-download-primary-support"
  "data-download-platform-button=\"windows\""
  "data-download-platform-button=\"macos\""
  "data-download-platform-button=\"linux\""
)

RELEASE_ASSET_URLS=(
  "https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-windows-x64.exe"
  "https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-macos-x64.dmg"
  "https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-linux-x86_64.AppImage"
  "https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-checksums.txt"
)

contains_fixed_pattern() {
  local pattern="$1"
  local file_path="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -Fq -- "${pattern}" "${file_path}"
    return $?
  fi
  grep -Fq -- "${pattern}" "${file_path}"
}

for entry in "${SITE_ENTRIES[@]}"; do
  [[ -f "${entry}" ]] || { echo "error: missing site entry: ${entry}" >&2; exit 1; }

  for url in "${RELEASE_ASSET_URLS[@]}"; do
    if ! contains_fixed_pattern "${url}" "${entry}"; then
      echo "error: missing release asset url in ${entry}: ${url}" >&2
      exit 1
    fi
  done

  for marker in "${REQUIRED_ENTRY_MARKERS[@]}"; do
    if ! contains_fixed_pattern "${marker}" "${entry}"; then
      echo "error: missing required download marker in ${entry}: ${marker}" >&2
      exit 1
    fi
  done
done

if ! contains_fixed_pattern "https://api.github.com/repos/eng-cc/oasis7/releases/latest" "${REPO_ROOT}/site/assets/app.js"; then
  echo "error: missing latest release api endpoint in site/assets/app.js" >&2
  exit 1
fi

echo "ok: site download entry and release links are present"
