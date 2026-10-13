# GitHub Pages Visual Content Refresh (2026-07-18)

- Owner role: `game_visual_interaction_designer`
- Original task: #2438 (`task_adfbfbfea1e64f4eb8904b9779b4c2bb`)
- Amendment: #2459 (`task_0f22c947c970496db3523537e0be7022`)
- Routes: `site/index.html`, `site/en/index.html`
- Generated asset: `site/assets/images/hero/asteroid-civilization-crisis-image2-20260719.webp`

## Decision

The current homepage layout is owned by the [Site root design](../design.md#homepage-layout-authority).
This supplement retains bilingual copy, responsive/accessibility acceptance and
asset provenance. Its former three-fact hero layout is historical, not a parallel
current requirement; the focused consequence layout supersedes only that layout.

Keep one pressure dilemma, concise player-role/consequence copy, one primary action,
a secondary text link, the exact preview status and the atmospheric world illustration.
Do not restore the three compact fact columns. Real Viewer captures remain evidence
and must not be replaced by generated illustration in proof-oriented contexts.
The [original decision](https://github.com/eng-cc/oasis7/blob/2adfc08706f1905c131adc0597a9ae5e99ceb2dd/doc/site/github-pages/github-pages-visual-content-refresh-2026-07-18.design.md)
remains available for historical provenance, not current layout or release approval.

## Comparable-site findings

- Strategy and survival game homepages usually lead with one fantasy or
  responsibility, one genre sentence, one primary action, and a dominant visual.
- Their deeper systems move below the first viewport instead of competing with
  the title.
- oasis7 keeps a stricter public-status boundary than released games: the exact
  `limited playable technical preview` label remains visible beside the hero.

## Content cuts

- Remove the tertiary overview link, two hero metric cards, and the three-row
  pulse panel.
- Reduce the first explainer from four overlapping definition cards to three
  beats: pressure, autonomous choice, persistent consequence.
- Keep downloads, detailed preview boundaries, docs, and evidence-chain truth
  in their existing downstream sections.

## Image2 prompt and boundary

Create a wide, dark sci-fi scene of a fractured asteroid-belt civilization.
Concentrate five to seven inhabited nodes, weathered industry, teal logistics
routes, and restrained amber pressure lights on the right; reserve calm dark
negative space on the left. Avoid text, UI, fake metrics, people, dominant
spacecraft, combat, explosions, logos, and screenshot framing.

The generated image is atmosphere, not evidence. Its repository-only provenance
remains recorded here and in the generated-asset path above; both language routes
present it as part of the world narrative, without an adjacent Image2 or
non-gameplay DOM caption. Meaningful copy must never be baked into the image.
The #2459 amendment removes player-facing production-process labels while
retaining this repository provenance and the exact public preview status.

## Visual acceptance

- At `1440x900`, the title, both CTAs, exact preview state, world art, and start
  of the next section are readable without overlap. The exact preview state, not
  the art, remains the availability boundary.
- At `390x844` and `360x800`, the header remains operable, CTAs keep 44px touch
  targets, English labels wrap naturally, and no horizontal overflow appears.
- Keyboard focus, skip navigation, no-JS navigation, and reduced-motion behavior
  remain intact.
- Browser screenshots are implementation evidence; the Image2 asset is only a
  hierarchy target and does not replace browser or QA verification.

## Traceability

### 2.1 需求承接与分配表

| 上游要求 | 分配内容 | 本设计条款 | 主责 | 限制 |
| --- | --- | --- | --- | --- |
| [首页布局与预览边界](../design.md#homepage-layout-authority) | 聚焦首屏、从属说明与唯一布局优先关系；保留双语、响应式和素材边界 | [决策与承接范围](#decision) | game_visual_interaction_designer；Site 维护者 | 不改页面实现或公开能力，不以日期替代专业决策 |

### 11.1 验证映射表

| 上游要求 | 本设计条款 | 验收内容 | 验证来源 | 记录 | 限制 |
| --- | --- | --- | --- | --- | --- |
| [首页布局与预览边界](../design.md#homepage-layout-authority) | [决策与承接范围](#decision) | 同候选根设计与两个补充对三列摘要、主次操作、图注和预览状态给出一致要求 | [文档检查](../../../scripts/doc-governance-check.sh)与同候选三文逐条审读；实际页面改动按本文件视口、双语、键盘/no-JS/reduced-motion 验收 | 文档 diff 和实际检查输出；页面变化另留真实截图和行为结果 | 本次文本修订不构成浏览器、视觉通过、部署或发布证据 |
