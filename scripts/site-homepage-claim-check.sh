#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

ZH_ENTRY="${REPO_ROOT}/site/index.html"
EN_ENTRY="${REPO_ROOT}/site/en/index.html"
APP_JS="${REPO_ROOT}/site/assets/app.js"
STYLES="${REPO_ROOT}/site/assets/styles.css"

contains_fixed_pattern() {
  local pattern="$1"
  local file_path="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -Fq -- "${pattern}" "${file_path}"
    return $?
  fi
  grep -Fq -- "${pattern}" "${file_path}"
}

check_required_patterns() {
  local file_path="$1"
  shift
  local pattern
  for pattern in "$@"; do
    if ! contains_fixed_pattern "${pattern}" "${file_path}"; then
      echo "error: missing required homepage pattern in ${file_path}: ${pattern}" >&2
      return 1
    fi
  done
}

ZH_PATTERNS=(
  "class=\"skip-link\""
  "data-homepage-claim=\"preview-status\""
  "状态：技术预览（还不能玩）"
  "data-homepage-claim=\"default-web-entry\""
  "默认网页验证入口：software_safe"
  "data-homepage-claim=\"future-platform-boundary\""
  "当前还不是 creator-facing 的 mod / 模块平台。"
  "data-homepage-claim=\"download-boundary\""
  "这不是 hosted web join，也不是正式玩家发布；正式公告仍在准备中。"
  "data-homepage-claim=\"builder-feedback\""
  "builder 反馈"
  "og:image:alt"
  "twitter:image:alt"
)

EN_PATTERNS=(
  "class=\"skip-link\""
  "data-homepage-claim=\"preview-status\""
  "Status: technical preview, not playable yet"
  "data-homepage-claim=\"default-web-entry\""
  "Default web verification entry: software_safe"
  "data-homepage-claim=\"future-platform-boundary\""
  "not a creator-facing mod / modules platform yet."
  "data-homepage-claim=\"download-boundary\""
  "this is not a hosted web join or a public player launch, and formal announcement is still pending."
  "data-homepage-claim=\"builder-feedback\""
  "builder feedback"
  "og:image:alt"
  "twitter:image:alt"
)

APP_JS_PATTERNS=(
  "document.documentElement.setAttribute(\"data-js\", \"true\");"
)

STYLE_PATTERNS=(
  ".skip-link"
  "html[data-js=\"true\"] .nav"
  "html[data-js=\"true\"] .menu-button"
  ".developer-details"
  ".boundary-banner"
)

check_required_patterns "${ZH_ENTRY}" "${ZH_PATTERNS[@]}"
check_required_patterns "${EN_ENTRY}" "${EN_PATTERNS[@]}"
check_required_patterns "${APP_JS}" "${APP_JS_PATTERNS[@]}"
check_required_patterns "${STYLES}" "${STYLE_PATTERNS[@]}"

echo "ok: homepage claim/parity, metadata, and no-js navigation markers are present"
