# HTML Roadshow Deck Page Design

- Professional owner: `game_visual_interaction_designer`
- Surface id: `site-html-roadshow-deck`
- Page paths: `site/deck/index.html`, `site/deck/en/index.html`
- Owner module: `site`
- Image2 desktop target: `doc/site/deck/assets/html-roadshow-deck-desktop-image2-2026-06-19.png`
- Image2 mobile target: `doc/site/deck/assets/html-roadshow-deck-mobile-image2-2026-06-19.png`
- Historical task and archived process: [Task #1484](https://github.com/eng-cc/oasis7/issues/1484#issuecomment-6162411651) (`task_e02c8fe08ebb4f51887b116675f676c1`); no ignored local archive is required to follow this source.

## Purpose

The HTML roadshow deck lets a presenter or external collaborator open a fixed
sequence and move through the project thesis, gameplay distinction, current
proof, boundary, and next proof point.

The first screen must establish:

- `oasis7 / Roadshow Deck`;
- the category thesis;
- the current-stage boundary: `limited playable technical preview`;
- return/navigation affordances;
- language switching between the Chinese and English decks.

## Coverage

The Chinese and English deck pages share one design draft because they use the
same route family, Reveal shell, slide patterns, and CSS. This draft covers both:

- `site/deck/index.html`
- `site/deck/en/index.html`

The implementation must still account for language length differences:

- English body lines are longer and need more card width or line wrapping.
- Chinese cards can be denser, but should not become paragraph walls.
- Shared controls, status chips, and slide navigation must remain readable in
  both languages.

## Image2 Generation Brief

Desktop prompt used:

```text
Create a polished desktop HTML roadshow deck design target for oasis7, 16:9 presentation page at 1440x900. It is a browser-based reveal.js style deck, not a marketing landing page. Dark space-industrial sci-fi visual system with restrained teal accents, warm amber boundary notes, sharp readable cards, and fixed chrome. Show top pill navigation with oasis7 brand and links: 首页, 文档中心, 项目总览, English Deck. Show bottom-left status chips: “HTML Deck” and “limited playable technical preview”. Main slide is the cover: kicker “oasis7 / Roadshow Deck”, headline “一个会自己推进历史的 AI Agent 文明模拟游戏”, body copy explaining the player sets goals and constraints in controlled preview scenarios, three metric cards for 题材, 玩家位置, 当前阶段, and a footer note that the deck does not replace homepage boundary or formal spec truth. Include subtle progress bar, slide number, and arrow controls. Text-heavy layout, no fake gameplay screenshots, no claim that the game is fully launched or broadly playable.
```

Mobile prompt used:

```text
Create a mobile design target for the oasis7 HTML roadshow deck, 390x844. Browser presentation page with reveal.js-like controls. Show compact wrapped top navigation, status chips “HTML Deck” and “limited playable technical preview”, slide number/progress, and a readable one-column cover slide. Use the English/CN bilingual-ready visual system: dark sci-fi background, restrained teal plus warm amber boundary accents, clear cards, no text clipping. Include a second mini-state preview showing a later proof/boundary slide opened by hash navigation, with the same chrome and controls still visible. Do not imply public launch, closed beta, or full playability.
```

## Information Architecture

- Persistent chrome: oasis7 brand, return links, docs/project links, and
  language switch.
- Persistent status: `HTML Deck` plus `limited playable technical preview`.
- Reveal slide canvas: fixed sequence, readable slide cards, controls, progress,
  and hash navigation.
- Cover slide: category thesis and boundary.
- Middle slides: repeatable patterns for opportunity, agenda, problem, thesis,
  player loop, stack, proof, and path.
- End slide: return links and next-step boundary.

## Required States

| State | Design requirement |
| --- | --- |
| Default cover | Cover slide carries thesis, status, and navigation without requiring prior context. |
| Hash/deep link | Direct links to later slides keep chrome, status, controls, and slide position readable. |
| Navigation | Keyboard/touch/Reveal controls remain visible without competing with content. |
| Language | Chinese and English pages share visual system while handling different text lengths. |
| Fragment | Progressive reveal cards should not make a slide look empty before fragments appear. |
| Mobile | One-column slide content, wrapped chrome, usable controls, no clipping. |
| Dense content | Proof/roadmap slides preserve hierarchy instead of shrinking text into illegibility. |

## Visual Direction

- Use a dark space-industrial deck language with teal as support and warm amber
  for boundary/status notes.
- Avoid a one-hue dark teal wall; hierarchy should come from contrast, rhythm,
  card scale, and accent shifts.
- Keep the technical-preview boundary persistent.
- Do not present fake gameplay screenshots or imply full public launch,
  closed beta, or broad playability.

## Implementation Comparison Notes

Current `site/deck/*` already provides the Reveal shell, fixed chrome, status
chips, and bilingual route. Implementation review should compare:

- whether cover and dense slides avoid text clipping on mobile;
- whether Reveal controls/status overlap content at narrow widths;
- whether English cards keep comfortable line wrapping;
- whether direct hash navigation still shows status and context;
- whether status language stays `limited playable technical preview`.

## Non-Replacement Boundary

The Image2 targets define visual direction and comparison expectations only.
They do not replace real browser screenshots, responsive checks, keyboard/touch
navigation smoke, accessibility review, QA evidence, or PR review.

### 2.1 需求承接与分配表

| 上游 requirement | 承接义务 | 本设计条款 | 权威职责 | 边界 |
| --- | --- | --- | --- | --- |
| [Site 入口合同](../prd.md#site-page-requirements) | 双语固定 slide、导航与技术预览边界 | [页面目标](html-roadshow-deck-page-2026-06-19.design.md#required-states) | 页面比较目标；现行系统 authority 仍在 Site 主设计 | Image2 不是浏览器、a11y 或发布证据 |

### 11.1 验证映射表

| 上游 requirement | 本设计条款 | 独立 obligation | 验证方法 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Site 入口合同](../prd.md#site-page-requirements) | [页面目标](html-roadshow-deck-page-2026-06-19.design.md#required-states) | 双语固定 slide、导航与技术预览边界 | N/A: reason=本次只修历史来源和归因且页面代码/资产未改; scope=原 Image2 target 不提供当前浏览器或视觉 QA; owner_role=game_visual_interaction_designer; evidence_ref=https://github.com/eng-cc/oasis7/issues/1484#issuecomment-6162411651; re-evaluate=修改实际页面时按当前 Site 合同执行对应 headed/导航/状态测试 | 页面实施 PR 的同候选截图与真实操作结果 | 不表示新 visual-designer signoff或当前发布验收通过 |
