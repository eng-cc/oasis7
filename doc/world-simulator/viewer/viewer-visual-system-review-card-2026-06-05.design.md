# Viewer Visual System Review Card (2026-06-05 21:26:33 CST)

- task_uid: `task_a25bf76359be45719edfcda1759626d1`
- branch: `task/world-simulator-viewer-visual-design-spec`
- reviewer: Codex model visual review using repository screenshots
- surface: Viewer `software_safe` player-facing visual system
- locale: `en`, `zh`
- viewport set: desktop 1280x720, mobile 390x844

## Inputs
- Change goal: complete the Viewer visual-system large items: brand book,
  token governance, icon/status vocabulary, asset-language rules, stable visual
  DOM hooks, and expanded screenshot evidence.
- Expected visual contract: Viewer remains an industrial world command table:
  world and command path first, diagnostics demoted, fallback honest, action
  receipts clear, sparse pixel-world layers readable, and CJK/mobile text not
  clipped.
- Screenshots:
  - `output/playwright/viewer-visual-system/2026-06-05T13-26-33Z/desktop-player-first-screen.png`
  - `output/playwright/viewer-visual-system/2026-06-05T13-26-33Z/mobile-player-first-screen.png`
  - `output/playwright/viewer-visual-system/2026-06-05T13-26-33Z/diagnostics-demotion-collapsed.png`
  - `output/playwright/viewer-visual-system/2026-06-05T13-26-33Z/cjk-long-text-mobile.png`
  - `output/playwright/pixel-world-fragment-visual/2026-06-05T13-43-42-359Z/fragment-fallback-visual.png`
  - `output/playwright/pixel-world-fragment-visual/2026-06-05T13-43-42-359Z/action-receipt-visual.png`
  - `output/playwright/pixel-world-fragment-visual/2026-06-05T13-43-42-359Z/mobile-action-receipt-visual.png`
- Automated evidence:
  - `output/playwright/viewer-visual-system/2026-06-05T13-26-33Z/focus-keyboard-visible-probe.json`
  - `output/playwright/viewer-visual-system/2026-06-05T13-26-33Z/fallback-renderer-hook-probe.json`
  - `output/playwright/pixel-world-fragment-visual/2026-06-05T13-43-42-359Z/summary.json`
- Known out of scope: public marketing identity, external art production,
  runtime protocol, launcher/explorer redesign, and GitHub approval.

## Verdict
- verdict: `pass`
- confidence: `high`
- human escalation needed: `no`
- owner action: Keep the brand-system companion linked from module entrypoints
  and require the screenshot matrix for future large visual-system changes.

## Must-Pass Checks
| Check | Result | Evidence / Note |
| --- | --- | --- |
| First visual focus matches the task goal | pass | Desktop and mobile first screens lead with the world command desk and next-step cards. |
| Main subject remains readable | pass | World, objective, accepted intent, and next step are readable at 1280x720 and 390x844. |
| No major overlap, crop, or horizontal overflow | pass | Mobile screenshot has no horizontal rail overflow; probe reports `horizontalOverflow=false`. |
| UI state is honest against supplied state / DTO evidence | pass | Empty/sync state is presented as waiting; fallback and derived state hooks are explicit. |
| Action feedback / blocker / next step is visible when required | pass | Visual smoke covers action receipt and mobile action receipt screenshots. |
| Diagnostics or debug panels do not dominate the primary player path | pass | Diagnostics screenshot shows Runtime Diagnostics after the player path summary and as a collapsed summary surface. |
| Desktop and mobile preserve the same priority order | pass | Desktop three-column layout and mobile rail keep World/Targets/Command/Diagnostics ordering. |

## Findings
1. No blocking visual findings.
2. Brand-system governance is represented as a separate companion spec, which is
   easier to enforce than expanding the parent visual spec indefinitely.
3. Focus evidence is CSS/probe based rather than a successful keyboard
   screenshot because the in-app browser keypress probe did not advance focus
   reliably; the CSS rule covers `a`, `button`, `canvas`, `summary`, `input`,
   `textarea`, and `select` with a semantic `--color-focus-ring` token.

## Pixel-World Addendum
- agent_readability: pass; visual smoke keeps world command board and action
  receipt above renderer diagnostics.
- fragment_background_role: pass; visual smoke and tests keep Fragment terrain
  as background behind routes/anchors/agents.
- location_logic_anchor_role: pass; locations are treated as logic anchors.
- action_receipt_clarity: pass; action receipt screenshots show player-facing
  feedback.
- no_receipt_honesty: pass by automated UI test coverage; no-receipt remains
  `data-receipt-present=false` and `confidence=none`.

## What This Review Does Not Prove
- It does not validate public marketing identity or future external artwork.
- It does not prove real player comprehension.
- It does not replace runtime, security, auth, or GitHub approval gates.

## Residual Risk
- Dense live data with many real agents/locations may still need a future
  screenshot matrix once runtime provides richer snapshots.
- The focus proof is CSS/probe based; a future browser harness with reliable
  keyboard focus screenshots would make this evidence stronger.

## Escalation Notes
- escalation_reason: none
- requested_human_owner: none
- decision_needed_by: N/A
