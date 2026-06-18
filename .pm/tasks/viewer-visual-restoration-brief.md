# Viewer Visual Restoration Brief

- task_uid: `task_060e9de147ba4757ac29cf0fb7a15210`
- role: `game_visual_interaction_designer`
- scope: real screenshot comparison review for the effect-image-first viewer restoration flow
- evidence date: 2026-06-18

## Evidence Reviewed

- `.pm/tasks/viewer-visual-targets-manifest.md`
- `.pm/tasks/viewer-visual-gap-notes.md`
- `.pm/tasks/viewer-compare-overview.png`
- `.pm/tasks/viewer-compare-layout-desktop.png`
- `.pm/tasks/viewer-compare-layout-mobile.png`
- `.pm/tasks/viewer-compare-focus-desktop.png`
- `.pm/tasks/viewer-compare-focus-mobile.png`
- `.pm/tasks/viewer-compare-command-agent-chat.png`
- `.pm/tasks/viewer-compare-gameplay-diagnostics.png`
- `.pm/tasks/viewer-compare-hosted-login-gate.png`
- `.pm/tasks/viewer-compare-empty-world-recovery.png`

## Review Boundary

Generated target images are Image2 visual direction. They may contain invented app chrome, rich terrain, fictional wording, or unavailable controls. This review does not require copying those literal features and does not request fake token, wallet, automation, or unreleased gameplay capability. Findings below evaluate visual hierarchy, density, composition, mobile readability, and state expression only.

## Overall Verdict

The flow now has target images, actual browser screenshots, and comparison boards for all eight viewer states, so the process foundation is correct. The frontend is not yet visually restored against the targets. The largest remaining gap is that actual screenshots still read as a form-heavy operational shell first, while the target direction reads as a world/map-first game command viewer with command surfaces supporting the selected state.

## Must Fix

1. World Focus map readability is still below target.
   - Screenshot pairs: `.pm/tasks/viewer-compare-focus-desktop.png`, `.pm/tasks/viewer-compare-focus-mobile.png`.
   - Problem: the actual map is mostly a sparse green grid with small markers and thin route lines. The target reads as an industrial world surface first: terrain chunks, route, selected agent, blocker, fragment/resource context, and destination are visually apparent before panel text.
   - Implementation direction: enrich the existing fixture/world rendering layer with stronger non-fake visual encoding from available state: larger selected-agent marker, thicker route with waypoint nodes, clearer blocker and target pins, fragment/resource patches or cell clusters, subtle terrain bands, and a stronger map viewport crop. Keep labels real: `agent-0`, `Missing Material`, route/fragments counts, target/build smelter. Do not invent new actions.

2. Desktop and mobile shell first viewport must put world/action state above explanatory cards.
   - Screenshot pairs: `.pm/tasks/viewer-compare-layout-desktop.png`, `.pm/tasks/viewer-compare-layout-mobile.png`.
   - Problem: actual desktop and mobile are structurally sound, but first read is a large "Recover sustainable capability" form/card stack plus connection notice. The target makes the command board/map the central object, with objective and next action supporting it.
   - Implementation direction: in populated visual fixtures, move the world board/map higher and make it visibly dominant in the first viewport. Reduce hero card height, compress or conditionally demote connection warning, and keep objective/next step as compact HUD tiles around the map rather than a large document section.

3. Gameplay diagnostics capture is comparable but visually not restored to the target state.
   - Screenshot pair: `.pm/tasks/viewer-compare-gameplay-diagnostics.png`.
   - Problem: actual shows the default shell with details text visible, but the target is a power-user inspection state where gameplay details and runtime diagnostics are clearly expanded and sectioned. The actual still reads like the normal command desk, not a diagnostics-focused mode.
   - Implementation direction: make the `gameplay_diagnostics_expanded` fixture open and frame the details/diagnostics sections as the primary content for that screenshot. Put gameplay meaning above raw diagnostics, use compact metric/status rows, and ensure the screenshot shows expanded details without becoming a raw JSON wall.

## Should Fix

1. Agent command/chat should expose message flow as the primary command surface.
   - Screenshot pair: `.pm/tasks/viewer-compare-command-agent-chat.png`.
   - Problem: actual has truthful chat and auth status, but the screenshot still emphasizes general command desk/intent cards more than the target's right-column chat conversation and send affordance.
   - Implementation direction: for `agent_chat_history`, scroll or arrange the right command column so recent messages, textarea, send button, selected target, and prompt/auth badges are all visible together. Keep prompt overrides and asset/governance subordinate.

2. Mobile World Focus command sheet remains too tall and textual for the target feel.
   - Screenshot pair: `.pm/tasks/viewer-compare-focus-mobile.png`.
   - Problem: map band is present, but the bottom sheet still consumes the lower viewport and reads as a text panel before a tactile command surface.
   - Implementation direction: further cap the default sheet height or make its first state denser: selected target chips, one-line blocker/receipt, compact command entry, and a visible drag/expand affordance. Preserve no-overflow and touch target size.

3. Visual language is consistent but too monochrome and low-contrast in actual map/stage areas.
   - Screenshot pairs: layout desktop/mobile, focus desktop/mobile.
   - Problem: the actual UI uses the correct dark operational palette, but map/stage areas lack the target's amber/mint/red contrast rhythm and spatial landmarks.
   - Implementation direction: reserve mint for agent/route, amber for objective/next step, red-orange for blocker/receipt, and add small but clear landmark/terrain accents in the world board.

## Acceptable Residual Risk

1. Hosted login/access gate is acceptable for this round.
   - Screenshot pair: `.pm/tasks/viewer-compare-hosted-login-gate.png`.
   - Rationale: actual correctly behaves like a focused access gate, keeps real email/code login semantics, and avoids fake wallet or token affordances. It does not need to copy the generated target's imagined editor chrome.

2. Empty-world recovery is acceptable for this round.
   - Screenshot pair: `.pm/tasks/viewer-compare-empty-world-recovery.png`.
   - Rationale: actual truthfully communicates missing world snapshot/entities and recovery next step. It is visually distinct from selected-agent focus and should not be forced to look like a populated map.

3. Target/actual image dimensions and exact app chrome do not need pixel matching.
   - Screenshot pairs: all comparison boards.
   - Rationale: current evidence is qualitative visual restoration, not pixel diff. Future acceptance should use fixed browser viewports and actual screenshot pairs after each restoration patch.

## Suggested Restoration Order

1. Restore world/map readability in World Focus desktop and mobile.
2. Rework shell first viewport so the world board/map is the main object.
3. Make diagnostics expanded state visually distinct and genuinely inspection-focused.
4. Tune command/chat visibility and mobile bottom sheet density.
5. Recapture the eight actual screenshots and regenerate comparison boards before claiming visual acceptance.
