# GitHub Pages Homepage Page Design Target (2026-06-19)

- Owner role: `game_visual_interaction_designer`
- Covered pages: `site/index.html`, `site/en/index.html`
- Related PRD: `doc/site/prd.md`
- Historical design context: `doc/site/design.md`, `doc/site/github-pages/*.design.md`
- Related task: GitHub issue #1484
  (`task_e02c8fe08ebb4f51887b116675f676c1`; archived pre-migration task files
  live in `.pm/github-project-sync/task-archive.jsonl`)
- Image2 desktop target: `doc/site/github-pages/assets/github-pages-homepage-desktop-image2-2026-06-19.png`
- Image2 mobile target: `doc/site/github-pages/assets/github-pages-homepage-mobile-image2-2026-06-19.png`

## Attribution Boundary

The `game_visual_interaction_designer` homepage slice was dispatched for this
补充 step, but the runtime timed out and was shut down before delivering a
usable professional conclusion. This design target is therefore a TPM-integrated
fallback artifact generated from existing repo truth:

- `site/index.html`
- `site/en/index.html`
- `doc/site/prd.md`
- `doc/site/design.md`
- historical `doc/site/github-pages/*.design.md`

Do not cite this document as a fresh visual-designer-authored conclusion. It is
page-level Image2 design evidence with the fallback boundary recorded in the
task execution log.

## 1. Design Coverage

This page-level draft covers the bilingual GitHub Pages homepage route family:

- Chinese route: `site/index.html`
- English route: `site/en/index.html`

Both routes share one visual system, layout hierarchy, claim boundary, and
responsive component model. The English route needs extra allowance for longer
CTA labels, longer proof chips, and longer card headings. The Chinese route
needs dense proof/status language to remain scannable without becoming a wall of
labels.

The design target is the public first-contact homepage, not the roadshow deck,
docs hub, story reader, release page, or viewer runtime surface.

## 2. First Viewport Requirements

The first viewport must make these points visible without requiring a scroll:

- `oasis7` is an AI Agent civilization simulation game.
- The world is a fractured asteroid-belt civilization under resource pressure.
- The player is outside the civilization, shaping strategy and constraints
  rather than directly puppeting every unit.
- The current status is exactly `limited playable technical preview`.
- It is not an official launch, not a closed beta, and not a broad public
  `play now` promise.
- Primary action: understand the gameplay loop quickly.
- Secondary actions: inspect one event/proof chain, open the docs/deck/story
  path, or continue toward current preview evidence.
- A proof/status strip must remain near the hero copy so the availability
  boundary is not buried below marketing language.
- A hint of the next gameplay explanation section should be visible on desktop
  and mobile so the page reads as an explainer, not only a hero billboard.

## 3. Desktop Image2 Prompt

```text
Create a polished desktop web page design target for the oasis7 GitHub Pages homepage, 1440x1100. It is a bilingual-ready public homepage for an AI-agent civilization simulation game set in a fractured asteroid belt. Use Simplified Chinese as the visible language. The design must look like a real product/game homepage, not a slide deck and not a pure engineering dashboard. First viewport: sticky compact oasis7 nav with links for 先看玩法, 现在能做什么, 证据链, 开发预览, 路线, 文档中心, 长篇故事. Hero headline: 带着一支 AI Agent 文明，在破碎小行星带里建设、交易与治理. Supporting copy explains the player guides civilization direction from outside, agents handle harvesting, production, trade, cooperation, and governance under resource pressure. Primary CTA 30 秒看懂玩法, secondary CTA 看一局事件链, tertiary link 看白皮书式总览. Visible proof/status strip: 世界：破碎小行星带文明, 玩家：文明外部指挥者, 玩法：扩张 / 交易 / 协作 / 治理, 状态：limited playable technical preview. Right side shows an abstract but truthful world-state visual: asteroid belt civilization nodes, logistics routes, resource pressure, cooperation lines, no fake gameplay screenshot. Below the fold show the next section edge with cards for 资源先紧起来, 协作会先谈，再改约, 决定落地以后，世界会记账. Dark restrained sci-fi, teal accents plus warm boundary notes, readable dense content, not one-note blue, no launch/closed beta/play now claims.
```

## 4. Mobile Image2 Prompt

```text
Create a mobile web page design target for the oasis7 GitHub Pages homepage, 390x1400. Simplified Chinese. It is the public homepage for an AI-agent civilization simulation game in a fractured asteroid belt, not a landing-page ad and not an engineering dashboard. Show compact top bar with oasis7, hamburger, and language switch affordance. First screen: eyebrow AI Agent 文明模拟游戏, large readable hero headline 带着一支 AI Agent 文明，在破碎小行星带里建设、交易与治理, short copy explaining the player guides direction from outside while agents run production, trade, cooperation, and governance under pressure. Primary button 30 秒看懂玩法, secondary 看一局事件链, smaller text link 看白皮书式总览. Status/proof chips must wrap cleanly: 世界：破碎小行星带文明, 玩家：文明外部指挥者, 玩法：扩张 / 交易 / 协作 / 治理, 状态：limited playable technical preview. Include an abstract truthful world-state panel with asteroid nodes/logistics/cooperation lines, not a fake gameplay screenshot. Below the fold show the beginning of the next section with three compact cards: 资源先紧起来, 协作会先谈，再改约, 决定落地以后，世界会记账. Dark restrained sci-fi, high contrast readable Chinese text, teal plus warm amber boundary accent, no horizontal overflow, no launch/closed beta/play now claims.
```

## 5. Required States And Risks

- Desktop first viewport: top navigation, language switch, hero copy, primary
  CTA, secondary CTAs, proof/status chips, world constellation panel, and at
  least the top edge of the next section must be visible and non-overlapping.
- Mobile first viewport: header, language/menu affordance, hero title, primary
  CTA, compact secondary actions, proof/status chips, and next-section edge must
  stack without horizontal overflow.
- Long English labels must wrap naturally. Do not solve English copy length by
  shrinking text to unreadable sizes.
- Status chips must preserve `limited playable technical preview`; wording
  variants must not drift into `closed beta`, `launch`, `play now`, or broad
  public availability.
- Hero visuals may be abstract world-state instrumentation, but must not look
  like a verified gameplay screenshot unless sourced from actual runtime
  capture.
- The homepage may link to deck, docs, story, proof, and preview evidence, but
  must not let the deck replace homepage boundary truth or PRD truth.
- The future platform direction may be hinted below the first viewport, but the
  first viewport must not imply that creator-facing modules or a public mod
  platform are currently open.

## 6. Relationship To Older Design Documents

The older GitHub Pages design documents remain valid historical and topical
design evidence for site information architecture, CTA cleanup, content sync,
visual polish, release/download communication, and quality gates. This document
does not replace those records.

This document adds the missing page-level Image2 target required for the current
homepage route family. When implementation and historical docs disagree, use
the current PRD and this page-level target for first-viewport visual hierarchy
and claim-boundary validation, then use older topical design docs for the
specific subtopic they own.

## Implementation Comparison Notes

- Compare real browser screenshots at desktop, tablet, and phone widths against
  the Image2 targets for hierarchy, not exact pixels.
- Keep the existing static-site architecture; do not introduce a new frontend
  framework to satisfy this design target.
- This Image2 target is visual design evidence. It does not replace browser
  screenshots, local link checks, homepage claim parity checks, accessibility
  checks, or runtime verification.
