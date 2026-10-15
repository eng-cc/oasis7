# GitHub Pages Visual Content Refresh (2026-07-18)

- Owner role: `game_visual_interaction_designer`
- Original task: #2438 (`task_adfbfbfea1e64f4eb8904b9779b4c2bb`)
- Amendment: #2459 (`task_0f22c947c970496db3523537e0be7022`)
- Routes: `site/index.html`, `site/en/index.html`
- Generated asset: `site/assets/images/hero/asteroid-civilization-crisis-image2-20260719.webp`

<a id="decision"></a>
## Decision（目标与决策）

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

<a id="image2-prompt-and-boundary"></a>
## Image2 prompt and boundary（素材边界）

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

本表只补既有产品子链的身份与分配，不改变上文 dated Image2 目标、原专业义务或当前 root 优先关系。产品 requirement 的完整验收仍由现行官网系统设计与原 owner 承担；这里不复制当前页面实现、不恢复旧布局，也不产生新的视觉或发布结论。

| 上游要求 | 分配内容 | 本设计条款 | 主责 | 限制 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-4544302c-40be-4aa3-a557-21fe6f475c19；[REQ-SITE-DISCOVERY-001](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-001) | 产品001仅首屏玩家位置、当前边界与有效主路径；其余原专业义务独立保留：聚焦首屏、从属说明与唯一布局优先关系；保留双语、响应式和素材边界 | [决策与承接范围](#decision) | game_visual_interaction_designer；Site 维护者；原专业承接：[首页布局与预览边界](../design.md#homepage-layout-authority) | 不改页面实现或公开能力，不以日期替代专业决策 |
| Trace refs: O7-REQ-de38c7a9-3a48-418d-be60-2c527bc1e8eb；[REQ-SITE-DISCOVERY-006](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-006) | 双语换行、44px触控、无横向溢出、键盘/skip/no-JS/reduced-motion | [既有条款](#visual-acceptance) | game_visual_interaction_designer；viewer_engineer / qa_engineer；[现行接收条款](public-site-discovery-and-showcase.design.md#des-site-accessibility) | 只承接上述既有子范围；不证明完整产品 AC、当前浏览器、安装器或游戏验收 |
| Trace refs: O7-REQ-4b01d2d4-ec3a-4dbb-90f8-fca0c4b0a822；[REQ-SITE-DISCOVERY-002](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-002) | 生成图仅氛围，保留仓库来源；#2459生产标签豁免不取消素材性质/真实性约束 | [既有条款](#image2-prompt-and-boundary) | game_visual_interaction_designer；viewer_engineer / qa_engineer；[现行接收条款](public-site-discovery-and-showcase.design.md#des-site-evidence) | 只承接上述既有子范围；不证明完整产品 AC、当前浏览器、安装器或游戏验收 |

### 11.1 验证映射表

| 上游要求 | 本设计条款 | 验收内容 | 验证来源 | 记录 | 限制 |
| --- | --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-4544302c-40be-4aa3-a557-21fe6f475c19；[REQ-SITE-DISCOVERY-001](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-001) | [决策与承接范围](#decision) | 产品001仅首屏玩家位置、当前边界与有效主路径；其余原专业义务保持但不冒充产品001完整承接：同候选根设计与两个补充对三列摘要、主次操作、图注和预览状态给出一致要求；仅本条既有界面子范围，其他原专业义务继续保留 | 文档检查（`scripts/doc-governance-check.test.sh`）与同候选三文逐条审读；实际页面改动按本文件视口、双语、键盘/no-JS/reduced-motion 验收；现行有限产品子范围按[官网验证方法](public-site-discovery-validation.manual.md#外部浏览器验证)：S6-DISC-01：同候选 CN/EN 首页主路径、有效 CTA 和当前 preview 边界；另记录陌生访客对玩家位置的原话，作者自测不代签理解；同候选静态服务器与实际浏览器，本文未运行；上述三文/三列摘要一致性审读只核历史替代边界，当前仍不得恢复三列；不是用旧布局为现行产品001签收 | 文档 diff 和实际检查输出；页面变化另留真实截图和行为结果 | 本次文本修订不构成浏览器、视觉通过、部署或发布证据；原专业承接：[首页布局与预览边界](../design.md#homepage-layout-authority) |
| Trace refs: O7-REQ-de38c7a9-3a48-418d-be60-2c527bc1e8eb；[REQ-SITE-DISCOVERY-006](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-006) | [既有条款](#visual-acceptance) | 双语换行、44px触控、无横向溢出、键盘/skip/no-JS/reduced-motion | [现行验证方法](public-site-discovery-validation.manual.md#外部浏览器验证)：S6-DISC-06中与本旧条款交集的子范围：仅原 Visual acceptance 范围：同候选1440×900、390×844、360×800，标题/CTA/preview可读、CN/EN长标签换行、44px触控、无横向溢出、键盘焦点/skip/no-JS/reduced-motion；不扩展200%缩放或性能义务；沿用test_tier_required / 适用S6，记录同候选实际浏览器结果；本文未运行，不重启界面实现 | 实施 PR 记录 source/build、路由/语言、候选环境与逐项 pass/fail/blocked/not_run；素材来源另由 QA 复核 | 只承接上述既有子范围；不证明完整产品 AC、当前浏览器、安装器或游戏验收 |
| Trace refs: O7-REQ-4b01d2d4-ec3a-4dbb-90f8-fca0c4b0a822；[REQ-SITE-DISCOVERY-002](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-002) | [既有条款](#image2-prompt-and-boundary) | 生成图仅氛围，保留仓库来源；#2459生产标签豁免不取消素材性质/真实性约束 | [现行验证方法](public-site-discovery-validation.manual.md#内容发布与纠正)：S6-DISC-02：核对概念/说明性/真实记录类型与可公开来源；来源失效不升级证据，生成资产不冒充运行截图；test_tier_required / 适用 S6，同候选、适用路由与实际浏览器；仅验证本行子范围，本文未运行 | 实施 PR 记录 source/build、路由/语言、候选环境与逐项 pass/fail/blocked/not_run；素材来源另由 QA 复核 | 只承接上述既有子范围；不证明完整产品 AC、当前浏览器、安装器或游戏验收 |
