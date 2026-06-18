# Viewer Visual Target Gap Notes

Task: `task_060e9de147ba4757ac29cf0fb7a15210`
Date: 2026-06-18

## Scope

The viewer is a single `software_safe.html` SPA. The user-facing visual set is therefore state based rather than route based. This pass generated effect images first, then added reproducible frontend fixture states so every target has a paired real browser screenshot.

Generated target images are visual direction for composition, hierarchy, density, palette, and state emphasis. They are not runtime truth, and Image2-invented product labels, wallet flows, fake maps, or unreleased controls must not be copied into production UI.

## Target To Actual Matrix

| State | Target | Actual | Fixture URL |
| --- | --- | --- | --- |
| Desktop command desk shell | `.pm/tasks/viewer-target-layout-desktop.png` | `.pm/tasks/viewer-actual-layout-desktop.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=shell_selected_blocker` |
| Mobile command desk shell | `.pm/tasks/viewer-target-layout-mobile.png` | `.pm/tasks/viewer-actual-layout-mobile.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=shell_selected_blocker` |
| Desktop World Focus selected-blocker | `.pm/tasks/viewer-target-focus-selected-blocker-desktop.png` | `.pm/tasks/viewer-actual-focus-selected-blocker-desktop.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&pixel_world_visual_fixture=selected_blocker` |
| Mobile World Focus selected-blocker | `.pm/tasks/viewer-target-focus-selected-blocker-mobile.png` | `.pm/tasks/viewer-actual-focus-selected-blocker-mobile.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&pixel_world_visual_fixture=selected_blocker` |
| Agent command/chat | `.pm/tasks/viewer-target-command-agent-chat.png` | `.pm/tasks/viewer-actual-command-agent-chat.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=agent_chat_history` |
| Gameplay details and diagnostics | `.pm/tasks/viewer-target-gameplay-diagnostics.png` | `.pm/tasks/viewer-actual-gameplay-diagnostics.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=gameplay_diagnostics_expanded` |
| Hosted login/access gate | `.pm/tasks/viewer-target-hosted-login-gate.png` | `.pm/tasks/viewer-actual-hosted-login-gate.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=hosted_login_gate` |
| Empty-world recovery | `.pm/tasks/viewer-target-empty-world-recovery.png` | `.pm/tasks/viewer-actual-empty-world-recovery.png` | `software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=empty_world_recovery` |

## Current Alignment

- Desktop shell: acceptable first-pass alignment. The implemented UI keeps the generated direction of left navigation, central command desk, and right command/inspect rail while preserving real viewer labels and auth constraints.
- Mobile shell: acceptable but still dense. It has a clear top jump rail and stacked command desk, but the viewport captures only the top portion of a long page.
- Desktop World Focus: acceptable first-pass alignment. The map remains primary, blocker/objective/tick cards are readable, and the command surface is contained in the right rail.
- Mobile World Focus: shippable for comparison, not final visual polish. The command drawer leaves mission-map context visible, but it still dominates the lower half of the viewport.
- Agent command/chat: acceptable first-pass alignment. The authenticated fixture exposes chat-ready state, message input, prompt/auth status, and reply history without copying generated fake controls.
- Gameplay diagnostics: acceptable first-pass alignment. It reuses the command desk shell and exposes diagnostics/detail state for screenshot comparison; further polish can make diagnostics feel more distinct from the default shell.
- Hosted login gate: acceptable first-pass alignment. The modal-like access gate is centered, visually separated, and keeps real email/code login semantics.
- Empty-world recovery: acceptable first-pass alignment. The empty snapshot state is visible in navigation, command cards, and the right rail with clear recovery guidance.

## Must Fix Before Claiming This Pass Complete

- None observed in the captured screenshots. All eight actual screenshots render non-empty current-bundle UI at the expected dimensions.

## Residual Risks / Follow-Up Polish

- Image2 targets are more editorial and sometimes invent UI affordances. Further refinement should use the generated images for hierarchy and tone, not as literal wireframes.
- Mobile World Focus still needs a later polish pass if the desired target is a lighter command sheet with more map visible above the fold.
- Gameplay diagnostics could be made visually more diagnostic-specific in a future pass instead of sharing most of the default shell composition.
- This pass establishes target images, reproducible fixtures, and first-pass implementation screenshots. It is not a pixel-match completion claim.

## Restored Screenshot Pass

Date: 2026-06-18

Restored evidence was captured after the screenshot-comparison brief and implementation pass:

- Restored actuals: `.pm/tasks/viewer-restored-actual-*.png`
- Restored comparisons: `.pm/tasks/viewer-restored-compare-*.png`
- Restored overview: `.pm/tasks/viewer-restored-compare-overview.png`
- Probe output: `.pm/tasks/viewer-restored-screenshot-probes.json`

Observed restored changes:

- Desktop shell now puts the world command board and map in the first viewport. Probe evidence: `layout-desktop` map top is `209px`, no horizontal overflow.
- Mobile shell now shows the world board and map in the first viewport. Probe evidence: `layout-mobile` map top is `335px`, no horizontal overflow.
- World Focus screenshots now explicitly enter focus mode before capture. Probe evidence: desktop and mobile `bodyFocus=true`, `pixelFixture=selected_blocker`, and `focusComparable=true`.
- World Focus map readability improved with richer terrain patches, larger selected-agent marker, clearer route/path, resource/hotspot nodes, blocker/goal callouts, and target context.
- Gameplay diagnostics now opens details and runtime diagnostics as the primary capture state. Probe evidence: `detailsOpen=true`, `diagnosticsOpen=true`.

Remaining visual risk:

- Mobile World Focus command drawer is still visually heavy, though the probe shows it below the half-viewport cap at approximately `0.43` of viewport height. Treat this as follow-up polish rather than a blocker for the restored screenshot flow.
- The restored pass is qualitative visual restoration against fixed screenshot pairs, not a pixel-diff acceptance gate.

## Round 2 Alignment Pass

Date: 2026-06-18

Round 2 focused on the remaining polish after the restored screenshot PASS:

- Visual brief: `.pm/tasks/viewer-visual-alignment-round2-brief.md`
- Round 2 actuals: `.pm/tasks/viewer-round2-actual-*.png`
- Round 2 comparisons: `.pm/tasks/viewer-round2-compare-*.png`
- Round 2 overview: `.pm/tasks/viewer-round2-compare-overview.png`
- Probe output: `.pm/tasks/viewer-round2-screenshot-probes.json`

Observed Round 2 changes:

- Mobile World Focus command sheet is lighter and shorter. Probe evidence: drawer ratio is approximately `0.403`, down from the restored pass `0.4265`, with no horizontal overflow.
- Mobile World Focus sheet now elevates target, blocker, receipt, and send action as compact rows/chips before longer explanatory copy.
- `agent_chat_history` now reads chat-first in the right command rail. Probe evidence: chat panel top is `150px`, message history count is `3`, and input/send are visible in the viewport.
- Map/stage rhythm has stronger mint route/selected-agent accents, amber goal/resource markers, and red-orange blocker markers without adding fake actions.

Remaining visual risk:

- Round 2 still follows qualitative visual alignment, not pixel matching.
- The generated target remains richer and more illustrative than the real viewer. Further work should stay bounded to actual viewer state and avoid Image2-only capabilities.
