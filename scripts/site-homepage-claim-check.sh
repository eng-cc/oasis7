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
  python3 - "${file_path}" "${claim}" <<'PY'
import sys
from html.parser import HTMLParser

class SectionClaimParser(HTMLParser):
    def __init__(self, claim):
        super().__init__(convert_charrefs=True)
        self.claim = claim
        self.matches = []

    def handle_starttag(self, tag, attrs):
        if tag.lower() != "section":
            return
        values = {name.lower(): value or "" for name, value in attrs}
        if values.get("data-homepage-claim") == self.claim:
            classes = values.get("class", "").split()
            self.matches.append("hidden" in values or "hidden" in classes or values.get("aria-hidden") == "true")

parser = SectionClaimParser(sys.argv[2])
parser.feed(open(sys.argv[1], encoding="utf-8").read())
parser.close()
if not parser.matches:
    raise SystemExit(f"error: missing visible homepage section claim in {sys.argv[1]}: {sys.argv[2]}")
if any(parser.matches):
    raise SystemExit(f"error: homepage section claim is hidden in {sys.argv[1]}: {sys.argv[2]}")
PY
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

check_provenance_claim_css_visibility() {
  python3 - "${STYLES}" <<'PY'
import re
import sys

css = open(sys.argv[1], encoding="utf-8").read()
for selector, declarations in re.findall(r"([^{}]+)\{([^{}]*)\}", css):
    if "doc-entry-note" not in selector:
        continue
    if re.search(r"\bdisplay\s*:\s*none\b", declarations, flags=re.IGNORECASE):
        raise SystemExit(
            f"error: homepage provenance claim can be hidden by CSS selector: {selector.strip()}"
        )
PY
}

ZH_PATTERNS=(
  "class=\"skip-link\""
  "data-homepage-claim=\"preview-status\""
  "状态：limited playable technical preview"
  "不是正式玩家发布或公开网页加入"
  "data-homepage-claim=\"indirect-agency\""
  "你可以影响文明，但不能替 Agent 决定每一步。"
  "资源告急。"
  "文明会怎么选？"
  "你给方向。Agent 作选择。"
  "看一次文明危机"
  "文明制图 · 世界氛围图"
  "data-homepage-claim=\"consequence-story\""
  "data-evidence-mode=\"controlled-replay\""
  "data-homepage-claim=\"world-laws\""
  "这个世界只有几条不可违反的法律。"
  "时间只向前走"
  "没有什么凭空出现"
  "你不能替它们选择"
  "后果可以回看"
  "不是实时世界遥测"
  "data-homepage-claim=\"provenance-story\""
  "data-homepage-claim=\"telemetry-boundary\""
  "基于当前受控测试场景整理的示例性片段"
  "不是实时世界遥测"
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
  "not a public player launch or public web join"
  "data-homepage-claim=\"indirect-agency\""
  "You can influence a civilization, but you cannot decide every step for its Agents."
  "Resources are running out."
  "What will the civilization choose?"
  "You set the direction. Agents make the choices."
  "Watch a civilization crisis"
  "CIVILIZATION CARTOGRAPHY · WORLD ATMOSPHERE"
  "data-homepage-claim=\"consequence-story\""
  "data-evidence-mode=\"controlled-replay\""
  "data-homepage-claim=\"world-laws\""
  "This world has only a few laws that cannot be broken."
  "Time only moves forward"
  "Nothing appears from nowhere"
  "You cannot choose for them"
  "Consequences can be revisited"
  "not live world telemetry"
  "data-homepage-claim=\"provenance-story\""
  "data-homepage-claim=\"telemetry-boundary\""
  "An illustrative excerpt based on current controlled test scenarios"
  "not live world telemetry"
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
  "普通浏览与刷新不会抹掉已发生的变化；治理恢复属于单独的审计流程。" \
  "每一份资源都有来源和代价" \
  "不知道的状态就标为未知" \
  "doc/product/world-rules-core-gameplay/prd.md"
check_required_patterns "${EN_ENTRY}" \
  "data-homepage-claim=\"world-laws\"" \
  "World laws" \
  "Ordinary browsing and refreshes do not erase changes that already happened; governed recovery follows a separate audited process." \
  "Every resource has a source and a cost" \
  "Unknown states stay visibly unknown" \
  "doc/product/world-rules-core-gameplay/prd.md"
check_forbidden_patterns "${ZH_ENTRY}" \
  "实时遥测流" \
  "实时世界状态" \
  "覆盖：离线回放 + 在线运行 + 审计校验" \
  "看一条受控后果链" \
  "静态制图 · 受控场景来源" \
  "资源变化必须来自被授权的因果链" \
  "思考也有代价" \
  "本页覆盖：说明性回放 + 因果读法 + 来源绑定边界"
check_forbidden_patterns "${EN_ENTRY}" \
  "live telemetry feed" \
  "live world state" \
  "Coverage: replay + live runtime + audit trace" \
  "Watch one controlled consequence chain" \
  "Static cartography · controlled-scenario source" \
  "Resource changes must come from an authorized causal source/sink" \
  "thinking has a cost" \
  "On-page coverage: illustrative replay + causal reading + source-binding boundary"
check_required_patterns "${APP_JS}" "${APP_JS_PATTERNS[@]}"
check_required_patterns "${STYLES}" "${STYLE_PATTERNS[@]}"
check_provenance_claim_css_visibility

# Keep the two public homepage entries equivalent at the structural level. The
# fixed-string checks above protect player-facing wording; this parser protects
# the DOM contract that wording alone cannot see (classes, proof panels,
# disclosures, and evidence provenance semantics).
python3 - "${ZH_ENTRY}" "${EN_ENTRY}" "${REPO_ROOT}/site" <<'PY'
from __future__ import annotations

import os
import sys
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


class Node:
    def __init__(self, tag: str, attrs: dict[str, str], ancestors: tuple["Node", ...]) -> None:
        self.tag = tag
        self.attrs = attrs
        self.ancestors = ancestors
        self.text: list[str] = []

    def classes(self) -> set[str]:
        return set(self.attrs.get("class", "").split())

    def text_content(self) -> str:
        return "".join(self.text)


class HomepageParser(HTMLParser):
    void_tags = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"}

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.nodes: list[Node] = []
        self.stack: list[Node] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        normalized = {name.lower(): value or "" for name, value in attrs}
        node = Node(tag.lower(), normalized, tuple(self.stack))
        self.nodes.append(node)
        if node.tag not in self.void_tags:
            self.stack.append(node)

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.handle_starttag(tag, attrs)
        if self.stack and self.stack[-1].tag == tag.lower():
            self.stack.pop()

    def handle_endtag(self, tag: str) -> None:
        tag = tag.lower()
        for index in range(len(self.stack) - 1, -1, -1):
            if self.stack[index].tag == tag:
                del self.stack[index:]
                return

    def handle_data(self, data: str) -> None:
        for node in self.stack:
            node.text.append(data)


def parse_homepage(path: Path) -> HomepageParser:
    parser = HomepageParser()
    parser.feed(path.read_text(encoding="utf-8"))
    parser.close()
    return parser


def descendants(parser: HomepageParser, root: Node) -> list[Node]:
    return [node for node in parser.nodes if node is root or root in node.ancestors]


def class_count(nodes: list[Node], class_name: str) -> int:
    return sum(class_name in node.classes() for node in nodes)


def is_template_node(node: Node) -> bool:
    return node.tag == "template" or any(ancestor.tag == "template" for ancestor in node.ancestors)


def stylesheet_signature(parser: HomepageParser, document_path: Path, site_root: Path) -> tuple[str, str] | None:
    links = [
        node
        for node in parser.nodes
        if node.tag == "link" and "stylesheet" in node.attrs.get("rel", "").lower().split()
    ]
    if len(links) != 1:
        return None
    href = links[0].attrs.get("href", "")
    parsed = urlsplit(href)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return None
    if parsed.path.startswith("/"):
        path = Path(os.path.normpath(os.path.abspath(site_root / parsed.path.lstrip("/"))))
    else:
        path = Path(os.path.normpath(os.path.abspath(document_path.parent / parsed.path)))
    try:
        relative_path = path.relative_to(site_root.resolve())
    except ValueError:
        return None
    version = parse_qs(parsed.query).get("v", [""])[0]
    return str(relative_path), version


def proof_signature(parser: HomepageParser) -> tuple[Node | None, dict[str, object]]:
    proof_roots = [node for node in parser.nodes if node.tag == "section" and node.attrs.get("id") == "proof"]
    if len(proof_roots) != 1:
        return None, {}
    root = proof_roots[0]
    nodes = [node for node in descendants(parser, root) if not is_template_node(node)]
    tabs = Counter(node.attrs.get("data-proof-tab", "") for node in nodes if "data-proof-tab" in node.attrs)
    panels = Counter(node.attrs.get("data-proof-panel", "") for node in nodes if "data-proof-panel" in node.attrs)
    events = Counter(node.attrs.get("data-proof-event", "") for node in nodes if "data-proof-event" in node.attrs)
    figures = [node for node in nodes if "proof-figure" in node.classes()]
    codes = [node for node in nodes if "data-proof-code" in node.attrs]
    causal_cards = [node for node in nodes if "proof-causal-card" in node.classes()]
    dynamic_causal = [node for node in nodes if "data-proof-causal" in node.attrs]
    dynamic_causal_nodes = descendants(parser, dynamic_causal[0]) if len(dynamic_causal) == 1 else []
    dynamic_causal_nodes = [node for node in dynamic_causal_nodes if not is_template_node(node)]
    causal_coverage = set(node.attrs.get("data-proof-panel", "") for node in causal_cards)
    if len(dynamic_causal) == 1 and len(causal_cards) <= 1:
        causal_coverage = set(tabs)
    figure_panels = Counter(node.attrs.get("data-proof-panel", "") for node in figures)
    code_panels = Counter(node.attrs.get("data-proof-panel", "") for node in codes)
    figure_visibility = Counter(
        (node.attrs.get("data-proof-panel", ""), node.attrs.get("data-proof-visible", "")) for node in figures
    )
    code_visibility = Counter(
        (node.attrs.get("data-proof-panel", ""), node.attrs.get("data-proof-visible", "")) for node in codes
    )
    event_visibility = Counter(
        (node.attrs.get("data-proof-event", ""), node.attrs.get("data-proof-visible", ""))
        for node in nodes
        if "data-proof-event" in node.attrs
    )
    features = {
        "figures": len(figures),
        "figure-panels": figure_panels,
        "code-panels": code_panels,
        "figure-visibility": figure_visibility,
        "code-visibility": code_visibility,
        "event-visibility": event_visibility,
        "causal": len(dynamic_causal),
        "causal-root-hidden": any(is_hidden(node) for node in dynamic_causal),
        "causal-cards": len(causal_cards),
        "causal-coverage": tuple(sorted(causal_coverage)),
        "causal-lists": class_count(nodes, "proof-causal-list"),
        "causal-kicker": sum("data-proof-causal-kicker" in node.attrs for node in dynamic_causal_nodes),
        "causal-title": sum("data-proof-causal-title" in node.attrs for node in dynamic_causal_nodes),
        "causal-steps": sum("data-proof-causal-step" in node.attrs for node in dynamic_causal_nodes),
        "log-details": sum(node.tag == "details" and "proof-log-details" in node.classes() for node in nodes),
        "switcher": sum("data-proof-switcher" in node.attrs for node in nodes),
        "controls": sum("data-proof-controls" in node.attrs for node in nodes),
        "timeline": sum("data-proof-timeline" in node.attrs for node in nodes),
        "evidence-mode": Counter(node.attrs.get("data-evidence-mode", "") for node in nodes if "data-evidence-mode" in node.attrs),
    }
    return root, {"nodes": nodes, "tabs": tabs, "panels": panels, "events": events, "features": features}


def is_hidden(node: Node) -> bool:
    return "hidden" in node.attrs or "hidden" in node.classes() or node.attrs.get("aria-hidden") == "true"


def check_page(path: Path, language: str, site_root: Path) -> tuple[dict[str, tuple[str, ...]], dict[str, int], tuple[Node | None, dict[str, object]], list[str]]:
    parser = parse_homepage(path)
    errors: list[str] = []
    landing_sections: dict[str, tuple[str, ...]] = {}
    for node in parser.nodes:
        if node.tag == "section" and node.attrs.get("id"):
            landing = tuple(sorted(item for item in node.classes() if item.startswith("landing-")))
            if landing:
                landing_sections[node.attrs["id"]] = landing

    required_landing_ids = {"hero", "proof", "world-laws", "world", "download"}
    missing_landing = sorted(required_landing_ids - landing_sections.keys())
    if missing_landing:
        errors.append(f"{path}: missing landing section classes for: {', '.join(missing_landing)}")

    claims = Counter(node.attrs.get("data-homepage-claim", "") for node in parser.nodes if "data-homepage-claim" in node.attrs)
    required_claims = {
        "preview-status",
        "indirect-agency",
        "consequence-story",
        "provenance-story",
        "world-laws",
        "telemetry-boundary",
        "access-signing-boundary",
        "default-web-entry",
        "future-platform-boundary",
        "download-boundary",
        "builder-feedback",
    }
    missing_claims = sorted(required_claims - claims.keys())
    if missing_claims:
        errors.append(f"{path}: missing disclosure claims: {', '.join(missing_claims)}")
    hidden_claims = [
        node.attrs.get("data-homepage-claim", "")
        for node in parser.nodes
        if "data-homepage-claim" in node.attrs and is_hidden(node)
    ]
    if hidden_claims:
        errors.append(f"{path}: disclosure claims are hidden: {', '.join(sorted(hidden_claims))}")

    proof_root, proof = proof_signature(parser)
    if proof_root is None:
        errors.append(f"{path}: expected exactly one #proof section")
    else:
        features = proof["features"]
        assert isinstance(features, dict)
        if features["switcher"] != 1 or features["controls"] != 1 or features["timeline"] != 1:
            errors.append(f"{path}: proof switcher/control/timeline markers must each occur once")
        if features["evidence-mode"] != Counter({"controlled-replay": 1}):
            errors.append(f"{path}: proof must declare data-evidence-mode=controlled-replay exactly once")
        if features["figures"] != len(proof["tabs"]):
            errors.append(f"{path}: proof must provide one proof figure per evidence tab")
        dynamic_causal = features["causal"] == 1 and features["causal-kicker"] == 1 and features["causal-title"] == 1 and features["causal-steps"] == 6
        static_causal = features["causal-cards"] == len(proof["tabs"]) and features["causal-coverage"] == tuple(sorted(proof["tabs"]))
        if features["causal-root-hidden"]:
            errors.append(f"{path}: proof causal controller must be the visible causal explanation, not a hidden marker")
        if not dynamic_causal and not static_causal:
            errors.append(f"{path}: proof causal explanation markers are incomplete")
        elif features["causal-lists"] != (1 if dynamic_causal else features["causal-cards"]):
            errors.append(f"{path}: each proof causal explanation must contain one causal step list")
        if features["log-details"] != 1:
            errors.append(f"{path}: proof must provide one expandable event-log disclosure")
        if any("data-proof-visible" not in node.attrs for node in proof["nodes"] if "data-proof-panel" in node.attrs):
            errors.append(f"{path}: every proof panel must declare data-proof-visible")

    provenance = [
        node
        for node in parser.nodes
        if node.attrs.get("data-homepage-claim") == "provenance-story"
    ]
    if len(provenance) != 1:
        errors.append(f"{path}: expected exactly one provenance-story disclosure")
    else:
        provenance_node = provenance[0]
        semantics = provenance_node.attrs.get("data-evidence-provenance", "")
        if semantics not in {"illustrative", "source-bound"}:
            errors.append(
                f"{path}: provenance-story must declare data-evidence-provenance=illustrative or source-bound"
            )
        elif semantics == "illustrative":
            expected_word = "示例性" if language == "zh" else "illustrative"
            if expected_word not in provenance_node.text_content().lower():
                errors.append(f"{path}: illustrative proof must visibly disclose {expected_word}")
        elif not provenance_node.attrs.get("data-evidence-source", "").strip():
            errors.append(f"{path}: source-bound proof must identify data-evidence-source")

    return landing_sections, dict(claims), (proof_root, proof), errors


zh_path = Path(sys.argv[1])
en_path = Path(sys.argv[2])
site_root = Path(sys.argv[3]).resolve()
zh_landing, zh_claims, zh_proof, errors = check_page(zh_path, "zh", site_root)
en_landing, en_claims, en_proof, en_errors = check_page(en_path, "en", site_root)
errors.extend(en_errors)

if zh_landing != en_landing:
    errors.append(f"homepage landing class parity mismatch: zh={zh_landing!r} en={en_landing!r}")
if zh_claims != en_claims:
    errors.append(f"homepage disclosure parity mismatch: zh={zh_claims!r} en={en_claims!r}")

zh_styles = stylesheet_signature(parse_homepage(zh_path), zh_path, site_root)
en_styles = stylesheet_signature(parse_homepage(en_path), en_path, site_root)
if zh_styles is None or en_styles is None or zh_styles != en_styles or not zh_styles[1]:
    errors.append(
        f"homepage stylesheet cache-busting mismatch or missing version: zh={zh_styles!r} en={en_styles!r}"
    )

if zh_proof[0] is not None and en_proof[0] is not None:
    zh_data = zh_proof[1]
    en_data = en_proof[1]
    for key in ("tabs", "events"):
        if zh_data[key] != en_data[key]:
            errors.append(f"homepage proof {key} parity mismatch: zh={zh_data[key]!r} en={en_data[key]!r}")
    zh_features = zh_data["features"]
    en_features = en_data["features"]
    for key in (
        "figures",
        "figure-panels",
        "code-panels",
        "figure-visibility",
        "code-visibility",
        "event-visibility",
        "causal-coverage",
        "log-details",
    ):
        if zh_features[key] != en_features[key]:
            errors.append(f"homepage proof feature parity mismatch for {key}: zh={zh_features[key]!r} en={en_features[key]!r}")
    if zh_features["evidence-mode"] != en_features["evidence-mode"]:
        errors.append("homepage proof evidence-mode parity mismatch")

if errors:
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    raise SystemExit(1)

print("ok: homepage structural, proof, disclosure, provenance, and stylesheet parity are present")
PY

echo "ok: homepage claim/parity, metadata, and no-js navigation markers are present"
