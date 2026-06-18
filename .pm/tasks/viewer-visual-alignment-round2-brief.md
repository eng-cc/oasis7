# Viewer Visual Alignment Round 2 Brief

- task_uid: `task_060e9de147ba4757ac29cf0fb7a15210`
- role: `game_visual_interaction_designer`
- scope: small visual delta after restored screenshot PASS
- request: `再对齐一遍`
- evidence date: 2026-06-18

## Evidence Reviewed

- `.pm/tasks/viewer-restored-compare-overview.png`
- `.pm/tasks/viewer-restored-compare-focus-mobile.png`
- `.pm/tasks/viewer-restored-compare-command-agent-chat.png`
- `.pm/tasks/viewer-restored-compare-layout-desktop.png`
- `.pm/tasks/viewer-restored-compare-layout-mobile.png`
- `.pm/tasks/viewer-restored-screenshot-probes.json`
- `.pm/tasks/viewer-visual-restoration-brief.md`
- `.pm/tasks/task_060e9de147ba4757ac29cf0fb7a15210.execution.md`

## Boundary

The restored pass has no blocking visual issue. Round 2 should be a narrow alignment polish, not a rewrite. Do not copy Image2-only app chrome, fictional terrain fidelity, token/wallet controls, fake automation, or unavailable gameplay commands. Keep all changes tied to current viewer state, existing fixture data, and browser probes.

## Must Fix For Round 2

1. Make Mobile World Focus command sheet feel lighter while keeping it touch-safe.
   - Evidence: `.pm/tasks/viewer-restored-compare-focus-mobile.png`; probe `drawerViewportRatio=0.4265`, `hasHorizontalOverflow=false`.
   - Current issue: the sheet is below half viewport, so it passes the previous blocker, but visually it still reads as a stacked explanatory panel. The target reads as a compact tactical command tray with clear action chips and message flow.
   - Direction:
     - Keep default sheet height at or below the current ratio; do not increase it.
     - Compress explanatory copy in the first visible sheet state.
     - Elevate selected target, blocker, receipt, and one primary command/chat affordance into compact chips or short rows.
     - Keep textarea visible only if it does not push the sheet into a text wall; otherwise use a compact input preview plus clear expand affordance.
     - Preserve touch targets and no horizontal overflow.

2. Make `agent_chat_history` read as chat-first rather than generic command desk.
   - Evidence: `.pm/tasks/viewer-restored-compare-command-agent-chat.png`.
   - Current issue: the fixture is valid and truthful, but the actual screenshot still gives primary weight to the world board and general command cards. The target's core is selected agent, ready badges, message history, input, and send affordance.
   - Direction:
     - In the `agent_chat_history` fixture, visually prioritize the right command/inspect rail or chat surface.
     - Put selected target, chat-ready/limited badges, recent message flow, input box, and send button in the first visible command area.
     - Reduce prominence of prompt/auth capability chips and generic world-board cards for this state.
     - Keep asset/governance/authorization boundaries visible but subordinate, not a competing panel.

3. Sharpen map/stage color rhythm and landmarks without adding fake features.
   - Evidence: `.pm/tasks/viewer-restored-compare-layout-desktop.png`, `.pm/tasks/viewer-restored-compare-layout-mobile.png`, `.pm/tasks/viewer-restored-compare-focus-mobile.png`.
   - Current issue: restored maps are materially better, but the target still has clearer mint route/agent, amber objective/resource, and red-orange blocker rhythm. Actual maps can look slightly low-contrast and abstract at mobile size.
   - Direction:
     - Reserve mint for selected agent, route, and active command path.
     - Reserve amber for objective/next-step/goal landmark.
     - Reserve red-orange for blocker/action receipt/error hotspot.
     - Add or strengthen small truthful landmarks from fixture data: goal marker, blocker marker, route waypoint, resource/fragment marker, location anchor.
     - Improve mobile marker contrast and label legibility before adding more decorative terrain.

## Should Fix

1. Tighten mobile first viewport rhythm.
   - Evidence: `.pm/tasks/viewer-restored-compare-layout-mobile.png`.
   - Direction: reduce vertical dead air between the rail, board title, map, and objective cards; keep the board feeling like one interactive surface.

2. Make desktop shell map label hierarchy less timid.
   - Evidence: `.pm/tasks/viewer-restored-compare-layout-desktop.png`.
   - Direction: slightly increase contrast/weight for selected marker, route endpoints, and blocker/goal tags so they read before secondary renderer diagnostics.

3. Preserve current restored PASS conditions.
   - Evidence: `.pm/tasks/viewer-restored-screenshot-probes.json`.
   - Direction: after implementation, probes should still show no horizontal overflow; mobile focus sheet should remain below half viewport; `gameplay_diagnostics_expanded` should remain details/diagnostics open.

## Non-goals

- Do not rebuild the whole viewer shell.
- Do not reopen hosted login, empty-world recovery, or diagnostics as Round 2 blockers.
- Do not copy Image2's fictional app chrome, isometric terrain fidelity, wallet/token affordances, or fake command actions.
- Do not make mobile Focus hide the map to make chat prettier; the map remains the primary game state.
- Do not degrade the restored PASS screenshots just to chase pixel similarity.

## Suggested Acceptance Evidence

After Round 2 implementation, recapture at minimum:

- `.pm/tasks/viewer-round2-actual-focus-mobile.png`
- `.pm/tasks/viewer-round2-actual-command-agent-chat.png`
- `.pm/tasks/viewer-round2-actual-layout-desktop.png`
- `.pm/tasks/viewer-round2-actual-layout-mobile.png`
- `.pm/tasks/viewer-round2-screenshot-probes.json`

Acceptance should be qualitative: lighter mobile command tray, chat-first command fixture, clearer mint/amber/red-orange landmarks, no overflow, no fake features.
