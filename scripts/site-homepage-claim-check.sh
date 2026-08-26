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
    if rg -Fq -- "${pattern}" "${file_path}"; then
      return 0
    fi
    return 1
  fi
  if grep -Fq -- "${pattern}" "${file_path}"; then
    return 0
  fi
  return 1
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

check_visible_section_claim() {
  local file_path="$1"
  local claim="$2"
  local section_opening
  section_opening="$(rg -o -U '<section[^>]*>' "${file_path}" | rg -F "data-homepage-claim=\"${claim}\"" || true)"
  if [[ -z "${section_opening}" ]]; then
    echo "error: missing visible homepage section claim in ${file_path}: ${claim}" >&2
    return 1
  fi
  if printf '%s\n' "${section_opening}" | rg -Fq 'hidden'; then
    echo "error: homepage section claim is hidden in ${file_path}: ${claim}" >&2
    return 1
  fi
}

check_forbidden_patterns() {
  local file_path="$1"
  shift
  local pattern
  for pattern in "$@"; do
    if contains_fixed_pattern "${pattern}" "${file_path}"; then
      echo "error: forbidden fake-live-telemetry claim in ${file_path}: ${pattern}" >&2
      return 1
    fi
  done
}

ZH_PATTERNS=(
  "class=\"skip-link\""
  "data-homepage-claim=\"preview-status\""
  "状态：limited playable technical preview"
  "data-homepage-claim=\"indirect-agency\""
  "你可以影响文明，但不能替 Agent 决定每一步。"
  "data-homepage-claim=\"consequence-story\""
  "data-evidence-mode=\"controlled-replay\""
  "data-homepage-claim=\"world-laws\""
  "不是实时世界遥测"
  "data-homepage-claim=\"indirect-agency\""
  "你设定方向与约束，但不直接安排每个 Agent 的行动。"
  "data-homepage-claim=\"provenance-story\""
  "来源：受控验证场景回放"
  "事件后果：Agent 的选择已经改变世界状态"
  "证据可追溯：截图、事件日志与审计轨迹"
  "data-homepage-claim=\"telemetry-boundary\""
  "当前页面展示的是受控场景证据，不是实时遥测。"
  "data-homepage-claim=\"access-signing-boundary\""
  "当前公开入口仅用于技术预览验证，不是公开网页玩家入口。"
  "Windows 尚未补齐代码签名，macOS 尚未完成 notarization"
  "checksums 仅用于辅助校验，不替代平台信任链。"
  "data-homepage-claim=\"default-web-entry\""
  "默认网页验证入口：viewer"
  "data-homepage-claim=\"future-platform-boundary\""
  "当前还不是 creator-facing 的 mod / 模块平台。"
  "data-homepage-claim=\"download-boundary\""
  "这不是公开网页玩家入口，也不是正式玩家发布；正式公告仍在准备中。"
  "data-homepage-claim=\"builder-feedback\""
  "builder 反馈"
  "og:image:alt"
  "twitter:image:alt"
)

EN_PATTERNS=(
  "class=\"skip-link\""
  "data-homepage-claim=\"preview-status\""
  "Status: limited playable technical preview"
  "data-homepage-claim=\"indirect-agency\""
  "You can influence a civilization, but you cannot decide every step for its Agents."
  "data-homepage-claim=\"consequence-story\""
  "data-evidence-mode=\"controlled-replay\""
  "data-homepage-claim=\"world-laws\""
  "not live world telemetry"
  "data-homepage-claim=\"indirect-agency\""
  "You set direction and constraints, but you do not directly arrange every Agent action."
  "data-homepage-claim=\"provenance-story\""
  "Source: controlled validation scenario replay"
  "Consequence: an Agent choice has changed world state"
  "Auditable evidence: screenshot, event log, and audit trace"
  "data-homepage-claim=\"telemetry-boundary\""
  "This page shows controlled-scenario evidence, not live telemetry."
  "data-homepage-claim=\"access-signing-boundary\""
  "The public entry is for technical-preview validation only, not a public web player entry."
  "Windows is not code-signed yet, macOS is not notarized yet"
  "checksums are only an extra check, not a platform trust chain."
  "data-homepage-claim=\"default-web-entry\""
  "Default web verification entry: viewer"
  "data-homepage-claim=\"future-platform-boundary\""
  "not a creator-facing mod / modules platform yet."
  "data-homepage-claim=\"download-boundary\""
  "this is not a public web player entry or a public player launch, and formal announcement is still pending."
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
check_visible_section_claim "${ZH_ENTRY}" "world-laws"
check_visible_section_claim "${EN_ENTRY}" "world-laws"
check_required_patterns "${ZH_ENTRY}" \
  "data-homepage-claim=\"world-laws\"" \
  "世界规则" \
  "来自《世界规则与核心玩法 PRD》" \
  "间接能动性" \
  "资源变化必须来自被授权的因果链" \
  "后果可审计" \
  "doc/product/world-rules-core-gameplay/prd.md"
check_required_patterns "${EN_ENTRY}" \
  "data-homepage-claim=\"world-laws\"" \
  "World laws" \
  "World Rules & Core Gameplay PRD" \
  "indirect agency" \
  "Resource changes must come from an authorized causal source/sink" \
  "Consequences remain auditable" \
  "doc/product/world-rules-core-gameplay/prd.md"
check_forbidden_patterns "${ZH_ENTRY}" "实时遥测流" "实时世界状态"
check_forbidden_patterns "${EN_ENTRY}" "live telemetry feed" "live world state"
check_required_patterns "${APP_JS}" "${APP_JS_PATTERNS[@]}"
check_required_patterns "${STYLES}" "${STYLE_PATTERNS[@]}"

echo "ok: homepage claim/parity, metadata, and no-js navigation markers are present"
