#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

SITE_ENTRIES=(
  "${REPO_ROOT}/site/index.html"
  "${REPO_ROOT}/site/en/index.html"
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

  if ! contains_fixed_pattern "data-release-tag" "${entry}"; then
    echo "error: missing data-release-tag in ${entry}" >&2
    exit 1
  fi
  if ! contains_fixed_pattern "data-release-date" "${entry}"; then
    echo "error: missing data-release-date in ${entry}" >&2
    exit 1
  fi
  if ! contains_fixed_pattern "data-release-notes-link" "${entry}"; then
    echo "error: missing data-release-notes-link in ${entry}" >&2
    exit 1
  fi
done

if ! contains_fixed_pattern "https://api.github.com/repos/eng-cc/oasis7/releases/latest" "${REPO_ROOT}/site/assets/app.js"; then
  echo "error: missing latest release api endpoint in site/assets/app.js" >&2
  exit 1
fi

echo "ok: site download entry and release links are present"
