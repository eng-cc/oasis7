# Story Reader Page Design

- Professional owner: `game_visual_interaction_designer`
- Surface id: `site-story-reader`
- Page path: `site/story/index.html`
- Owner module: `site + story`
- Image2 desktop target: `doc/site/story/assets/story-reader-page-desktop-image2-2026-06-19.png`
- Image2 mobile target: `doc/site/story/assets/story-reader-page-mobile-image2-2026-06-19.png`
- Related task: `.pm/tasks/task_e02c8fe08ebb4f51887b116675f676c1.yaml`

## Purpose

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

## Information Architecture

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

## Visual Direction

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
