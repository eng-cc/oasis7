# Page Design Coverage

- Owner role: `game_visual_interaction_designer`
- Scope: current page-level public/player-facing HTML surfaces and explicit
  fixture/debug/evidence classifications.
- Related task: GitHub issue #1484
  (`task_e02c8fe08ebb4f51887b116675f676c1`; archived pre-migration task files
  live in `.pm/github-project-sync/task-archive.jsonl`)

## Coverage Rules

- Public/player/docs/ops/project pages need page-level design evidence.
- Bilingual route variants may share one design draft when the visual system,
  route family, and component model are shared; the draft must list all covered
  paths and language-length risks.
- Social export/generated pages may share package-level post-pack design
  evidence when each output maps to that package.
- Evidence/test fixture/debug-only HTML does not need full product-page design
  evidence when explicitly classified here or near the file.
- Vendor, third-party, pure build outputs, and non-user-entry fixtures are out of
  scope.

## Public Site Surfaces

| Surface | Page paths | Design evidence | Status |
| --- | --- | --- | --- |
| Public homepage | `site/index.html`, `site/en/index.html` | `doc/site/github-pages/github-pages-homepage-page-2026-06-19.design.md`; historical context in `doc/site/design.md` and `doc/site/github-pages/*.design.md` | Covered with Image2 page target |
| Docs hub and mirrors | `site/doc/cn/index.html`, `site/doc/en/index.html`, `site/doc/cn/project-overview.html`, `site/doc/en/project-overview.html`, `site/doc/cn/viewer-manual.html`, `site/doc/en/viewer-manual.html` | `doc/site/manual/*.design.md`, `doc/site/prd.md` | Covered |
| Story reader | `site/story/index.html` | `doc/site/story/story-reader-page-2026-06-19.design.md` | Covered |
| HTML roadshow deck | `site/deck/index.html`, `site/deck/en/index.html` | `doc/site/deck/html-roadshow-deck-page-2026-06-19.design.md` | Covered |
| XiaoHongShu social exports | `site/social/xiaohongshu/**/*.html` | Package README/post-pack markdown in each slug directory; export PNGs under `exports/` | Covered by package-level design evidence |

## Viewer / Launcher Surfaces

| Surface | Page paths | Design evidence | Status |
| --- | --- | --- | --- |
| Viewer software-safe page | `crates/oasis7_viewer/software_safe.html` | `doc/world-simulator/viewer/viewer-page-module-design-2026-06-18.design.md`, `doc/world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md` | Covered |
| Launcher app shell | `crates/oasis7_client_launcher/index.html` | `doc/world-simulator/launcher/*design.md`; launcher Image2/native screenshot workflow evidence in GitHub issue/task `task_54647d0add024a98b801d3736700ff22`, with retired pre-migration `.pm/tasks/*` evidence available only through `.pm/github-project-sync/task-archive.jsonl` | Covered |

## Explicit Fixture / Evidence Classification

| Path | Classification | Coverage note |
| --- | --- | --- |
| `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html` | `evidence/test fixture` | Iframe harness for `software_safe.html?test_api=1&connect=0` snapshot injection. It is not a product page surface and inherits Viewer design only as fixture context. |

## Residual Risk

This index is file/discovery based. It does not prove that implementation pixels
match design targets. For page changes, capture real desktop/mobile browser
screenshots, compare against the linked Image2 targets, and record gap notes in
the owning design draft or GitHub task issue evidence comments.
