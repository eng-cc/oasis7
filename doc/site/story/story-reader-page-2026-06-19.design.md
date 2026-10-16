# Story Reader Page Design

- Professional owner: `game_visual_interaction_designer`
- Surface id: `site-story-reader`
- Page path: `site/story/index.html`
- Owner module: `site + story`
- Image2 desktop target: `doc/site/story/assets/story-reader-page-desktop-image2-2026-06-19.png`
- Image2 mobile target: `doc/site/story/assets/story-reader-page-mobile-image2-2026-06-19.png`
- Historical task and archived process: [Task #1484](https://github.com/eng-cc/oasis7/issues/1484#issuecomment-6162411651) (`task_e02c8fe08ebb4f51887b116675f676c1`); no ignored local archive is required to follow this source.

<a id="purpose"></a>
## Purpose（页面目标）

The story reader page lets a reader or collaborator open the public story
workspace, understand that Volume 01 is a stable reading baseline, and start
reading without mistaking later drafts for final publication.

The first screen must communicate:

- `绿洲 2076`;
- Volume 01 is a reading version `v1.0-rc`;
- the stable reading range is `CH-001` to `CH-036`;
- this is not the final publication text;
- the primary next action is `开始阅读第一卷`.

## Image2 Generation Brief

Desktop prompt used:

```text
Create a polished desktop web page design target for the oasis7 story reader page, 1440x1100. The page is in Simplified Chinese and is a public story workspace for a longform sci-fi novel named “绿洲 2076”. Use a restrained literary sci-fi interface, quiet dark-neutral background, subtle paper/terminal texture, high readability, and calm accent colors. First viewport: small top navigation with “oasis7 story”, hero title “绿洲 2076”, subtitle explaining Volume 01 is a reading version v1.0-rc and not final publication, primary button “开始阅读第一卷”, secondary links “第一卷发布清单”, “打开世界背景”, “查看总纲”, “看正文索引”, plus a proof strip with “第一卷：阅读版 v1.0-rc”, “范围：CH-001 到 CH-036”, “边界：稳定阅读版，不等同最终出版定稿”. Below the fold show a story reader area: left chapter navigation grouped by volumes, right reading panel with loaded markdown chapter text, status line, generous line height, and no marketing claims. Make the layout text-heavy, realistic, browser-like, accessible, and avoid suggesting the product is fully launched or playable.
```

Mobile prompt used:

```text
Create a mobile web page design target for the oasis7 story reader page, 390x1200. Simplified Chinese. A calm literary sci-fi reading interface for “绿洲 2076”, focused on readable longform text. Show compact top bar, hero title, short reading-version boundary copy, a primary “开始阅读第一卷” button, wrapped secondary links, proof chips, then a stacked reader: chapter group buttons above a reading panel. Include a visible loading/error/empty-state style sample inside the reader panel as small muted status rows. Preserve the message that Volume 01 is a reading version v1.0-rc, not a final publication. Avoid launch, beta, or fully playable claims.
```

<a id="information-architecture"></a>
## Information Architecture（信息结构）

- Top chrome: quiet site navigation, `oasis7 story` brand, one reader anchor.
- Hero: story title, concise boundary copy, primary reader CTA, secondary
  reference links.
- Proof strip: reading version, chapter range, not-final-publication boundary.
- Reader section: heading, release note, and reader shell.
- Desktop reader shell: chapter navigation on the left and markdown reader panel
  on the right.
- Mobile reader shell: chapter groups stack above the reading panel; all buttons
  remain tap-safe and long Chinese labels wrap without horizontal scroll.

## Required States

| State | Design requirement |
| --- | --- |
| Default | Reader either loads the first chapter or invites chapter selection without visual emptiness. |
| Loading | Reader panel shows a calm loading state without layout jump. |
| Empty | Empty markdown or no chapter selected shows a repo-canon boundary note. |
| Error | Missing markdown or fetch failure gives a recoverable message and a direct source path/link. |
| Long content | Chapter body keeps comfortable line length, line height, and hierarchy during long reading. |
| Mobile | Chapter groups stack, buttons remain tappable, and reader text does not overflow. |

<a id="visual-direction"></a>
## Visual Direction（视觉决策）

- Use restrained literary sci-fi, not a product-launch landing page.
- Prefer quiet dark-neutral surfaces, readable text, and subtle paper/terminal
  texture.
- Keep the primary CTA visually dominant; secondary links should not bury the
  reading action.
- Keep public boundary copy visible but not bureaucratic.
- Avoid launch, beta, commercial publication, or complete-story claims.

## Implementation Comparison Notes

Current `site/story/index.html` already matches the broad IA: hero, CTA row,
proof strip, release note, chapter navigation, and reader panel are present.
Implementation review should compare:

- whether the primary reading CTA stays dominant after wrapping;
- whether chapter navigation remains scan-friendly with long chapter labels;
- whether markdown headings, body line length, and status messages keep the
  literary reading tone;
- whether loading, empty, and error states are visible and calm;
- whether mobile reader order is `hero -> proof -> chapter controls -> reader`.

## Non-Replacement Boundary

The Image2 targets define visual direction and comparison expectations only.
They do not replace real browser screenshots, responsive checks, interaction
smoke, accessibility review, QA evidence, or PR review.

### 2.1 需求承接与分配表

本表只补既有产品子链的身份与分配，不改变上文 dated Image2 目标、原专业义务或当前 root 优先关系。产品 requirement 的完整验收仍由现行官网系统设计与原 owner 承担；这里不复制当前页面实现、不恢复旧布局，也不产生新的视觉或发布结论。

这里只追踪现有入口/状态/恢复合同，不涉及界面重设计或小说正文。

| 上游 requirement | 承接义务 | 本设计条款 | 权威职责 | 边界 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-ee159d08-7998-44e2-bac5-930b7fa8a0ae；[REQ-SITE-DISCOVERY-005](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-005) | 故事入口、当前章节状态与失败/空白边界 | [页面目标](story-reader-page-2026-06-19.design.md#required-states) | 页面比较目标；现行系统 authority 仍在 Site 主设计；原专业承接：[Site 入口合同](../prd.md#story-entry-and-state-contract) | Image2 不是浏览器、a11y 或发布证据 |
| Trace refs: O7-REQ-de38c7a9-3a48-418d-be60-2c527bc1e8eb；[REQ-SITE-DISCOVERY-006](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-006) | 现有单语言 reader 的移动端堆叠、可读文字与可恢复状态；不制造英文正文或双语路由 | [既有条款](#required-states) | game_visual_interaction_designer；viewer_engineer / qa_engineer；[现行接收条款](../github-pages/public-site-discovery-and-showcase.design.md#des-site-accessibility) | 只承接上述既有子范围；不证明完整产品 AC、当前浏览器、安装器或游戏验收 |

### 11.1 验证映射表

| 上游 requirement | 本设计条款 | 独立 obligation | 验证方法 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-ee159d08-7998-44e2-bac5-930b7fa8a0ae；[REQ-SITE-DISCOVERY-005](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-005) | [页面目标](story-reader-page-2026-06-19.design.md#required-states) | 故事入口、当前章节状态与失败/空白边界；仅本条既有界面子范围，其他原专业义务继续保留 | [现行验证方法](../github-pages/public-site-discovery-validation.manual.md#外部浏览器验证)：S6-DISC-05：英文首访中文 reader，正常/空白/404/网络失败、当前章节重试、A→B 迟到成功与失败及同章重试缓存隔离；复用 root STORY-AC-1–5；test_tier_required / 适用 S6，同候选静态服务器与实际浏览器，本文未运行。原历史归因处置仅限当次来源修订：N/A: reason=本次只修历史来源和归因且页面代码/资产未改; scope=原 Image2 target 不提供当前浏览器或视觉 QA; owner_role=game_visual_interaction_designer; evidence_ref=https://github.com/eng-cc/oasis7/issues/1484#issuecomment-6162411651; re-evaluate=修改实际页面时按当前 Site 合同执行对应 headed/导航/状态测试 | 页面实施 PR 的同候选截图与真实操作结果 | 不表示新 visual-designer signoff或当前发布验收通过；原专业承接：[Site 入口合同](../prd.md#story-entry-and-state-contract) |
| Trace refs: O7-REQ-de38c7a9-3a48-418d-be60-2c527bc1e8eb；[REQ-SITE-DISCOVERY-006](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-006) | [既有条款](#required-states) | 现有单语言 reader 的移动端堆叠、可读文字与可恢复状态；不制造英文正文或双语路由 | [现行验证方法](../github-pages/public-site-discovery-validation.manual.md#外部浏览器验证)：S6-DISC-06中与本旧条款交集的子范围：仅现有中文单语言 reader：同候选手机章节组纵向堆叠、按钮可触达、中文长标签与正文不横向溢出且可读，当前状态及原文/重试入口可恢复；不新增CN/EN配对、英文正文、无JS、动效、缩放或性能义务；沿用test_tier_required / 适用S6，记录同候选实际浏览器结果；本文未运行，不重启界面实现 | 实施 PR 记录 source/build、路由/语言、候选环境与逐项 pass/fail/blocked/not_run；素材来源另由 QA 复核 | 只承接上述既有子范围；不证明完整产品 AC、当前浏览器、安装器或游戏验收 |
