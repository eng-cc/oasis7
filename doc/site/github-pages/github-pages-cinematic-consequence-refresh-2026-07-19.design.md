# GitHub Pages Cinematic Consequence Refresh (2026-07-19)

- Owner role: `game_visual_interaction_designer`
- Task: #2449 (`task_409c0a8f6d714a0293e308e290c4ac5d`)
- Routes: `site/index.html`, `site/en/index.html`
- Image2 target: `doc/site/github-pages/assets/github-pages-cinematic-consequence-image2-2026-07-19.png`
- Browser baseline: local static Pages at `1440x900` and `390x844`

## Decision

The [Site root design](../design.md#homepage-layout-authority) owns the current
homepage layout and precedence. This supplement supplies consequence rhythm and
progressive disclosure; it does not override the July 18 supplement's bilingual,
responsive, accessibility or repository-provenance obligations. The focused hero
supersedes the earlier three-fact layout only; no new browser or release acceptance
is implied by this documentation reconciliation.

Use **Cinematic consequence**. The homepage leads with one short resource-pressure
dilemma, one sentence defining player agency and persistence, one dominant event
chain action, the exact preview status, and one atmospheric world visual. The
next section turns the loop into an open three-beat rail instead of another card
grid.

The Image2 target is a hierarchy and rhythm reference. It is not a gameplay
capture, implementation proof, or a source of public claims. All meaningful copy,
status, actions and alt text remain in HTML. Under the #2459 amendment recorded in
[the visual-content supplement](github-pages-visual-content-refresh-2026-07-18.design.md#image2-prompt-and-boundary),
public Image2/non-gameplay production labels are not required. Repository provenance
and the exact preview status remain; generated art is still not gameplay evidence.

## Comparable-site findings

- Norland opens with a story-generating strategy promise, a dominant trailer or
  world visual, and a purchase action before its longer description.
- Factorio opens with one plain-language gameplay definition and immediate demo
  or purchase paths; deeper systems and community content follow later.
- Frostpunk and Dune: Awakening sell player burden and world fantasy before
  enumerating mechanics.
- Against the Storm likewise establishes role, hostile pressure, and settlement
  objective before feature detail.

The shared pattern is not "less information everywhere". It is one promise and
one action first, followed by progressive disclosure of systems and trust detail.
Oasis7 retains a stricter boundary because it is a limited playable technical
preview rather than a released game.

## Content and hierarchy changes

- Shorten the Chinese hero from a multi-clause choice list to
  `资源告急。文明会怎么选？`; keep the English route equivalent in intent.
- Make the event-chain action primary and demote the explanation action to a
  text link.
- Remove the three hero fact columns. Their meaning is already present in the
  headline, subtitle, and next section.
- Replace the first three bordered cards with an open consequence rail:
  pressure -> Agent response -> persistent result.
- Preserve the proof and download boundary sections below. Future compression of
  their trust-critical detail must use progressive disclosure, not deletion.

## Visual implementation brief

- Let the existing asteroid civilization art bleed behind the hero layout and
  use a strong dark-to-clear gradient to protect HTML copy.
- Reduce borders in the hero and first loop section; use spacing, scale, and a
  single rail to establish rhythm.
- On mobile, render promise and actions before the image. Maintain 44px actions,
  no horizontal overflow and a readable crop. Preserve HTML alt text and the preview status; do not reinstate the superseded production-label requirement.
- Do not add fake telemetry, fake metrics, autoplay, scroll-jacking, or claims
  baked into generated art.

## Acceptance

- At `1440x900`, the category, short H1, subtitle, dominant CTA, exact preview
  status, atmospheric visual, and start of the consequence rail are visible.
- At `390x844` and `360x800`, promise and CTA appear before the image, actions
  remain at least 44px high, and there is no horizontal overflow.
- Chinese and English routes retain the same sequence and preview boundary.
- Link, homepage claim/parity, manual sync, download, keyboard, no-JS, and
  reduced-motion checks remain valid.
- Real browser screenshots remain implementation evidence; the Image2 target
  does not replace them.

## Traceability

### 2.1 需求承接与分配表

| 上游要求 | 分配内容 | 本设计条款 | 主责 | 限制 |
| --- | --- | --- | --- | --- |
| [首页布局与预览边界](../design.md#homepage-layout-authority) | 聚焦首屏、从属说明与唯一布局优先关系；保留双语、响应式和素材边界 | [决策与承接范围](#decision) | game_visual_interaction_designer；Site 维护者 | 不改页面实现或公开能力，不以日期替代专业决策 |

### 11.1 验证映射表

| 上游要求 | 本设计条款 | 验收内容 | 验证来源 | 记录 | 限制 |
| --- | --- | --- | --- | --- | --- |
| [首页布局与预览边界](../design.md#homepage-layout-authority) | [决策与承接范围](#decision) | 同候选根设计与两个补充对三列摘要、主次操作、图注和预览状态给出一致要求 | [文档检查](../../../scripts/doc-governance-check.test.sh)与同候选三文逐条审读；实际页面改动按本文件视口、双语、键盘/no-JS/reduced-motion 验收 | 文档 diff 和实际检查输出；页面变化另留真实截图和行为结果 | 本次文本修订不构成浏览器、视觉通过、部署或发布证据 |
