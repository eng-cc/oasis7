# Viewer Visual Design Specification (2026-06-05)

- Professional owner: `game_visual_interaction_designer`
- Integration owner: `tpm`
- Source task uid: `task_a25bf76359be45719edfcda1759626d1` / GitHub issue #1385; execution evidence is in GitHub task issue evidence comments and `.pm/github-project-sync/task-archive.jsonl`.
- Related entrypoints:
  - `doc/world-simulator/viewer/viewer-brand-system-2026-06-05.design.md`
  - `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
  - `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md`
  - `doc/testing/manual/model-visual-review-sop-2026-05-29.manual.md`

## 1. Purpose
This document is the canonical visual design specification for Viewer and
player-facing world-simulator screens. It consolidates the current visual
direction that was previously spread across release experience, Viewer Web,
pixel-world, 2D readability, and visual review SOP documents.

The professional visual direction in this document is owned by the
`game_visual_interaction_designer` slice recorded under
`task_a25bf76359be45719edfcda1759626d1` / GitHub issue #1385. TPM integrated
that slice into the repository docs and does not replace the professional role's
visual judgment.

The specification answers:
- what a player should notice first;
- how world, agent, action, diagnostics, and command surfaces are layered;
- which visual rules are non-negotiable for new Viewer work;
- what evidence is required before a visual change can be considered ready.

For executable brand-system details, token taxonomy, icon/status vocabulary,
asset-language rules, and the expanded screenshot matrix, use
`viewer-brand-system-2026-06-05.design.md` as the companion specification.

## 2. Scope
In scope:
- Viewer Web and `software_safe` player-facing screens.
- Pixel-world stage, rendered DOM world surfaces, 2D map/readability overlays,
  right-side command surfaces, chat/prompt panels, receipts, blockers, empty
  states, loading states, and diagnostics disclosure.
- Site or documentation screenshots only when they represent the current Viewer
  experience.

Out of scope:
- Runtime rules, economy balance, networking, auth, chain state, or WASM ABI
  semantics.
- New large art packs, marketing campaign visuals, launcher/explorer redesigns,
  or public brand identity outside Viewer. Viewer-specific brand-system rules
  are in `viewer-brand-system-2026-06-05.design.md`.
- Real-player retention claims, external messaging, release approval, or
  GitHub required review.

## 3. Source Documents Consolidated
This specification consolidates and does not replace:
- `viewer-gameplay-release-experience-overhaul.prd.md`: Player mode,
  Director mode, world-first release experience, and command discoverability.
- `viewer-web-entry-visual-redesign-2026-05-12.prd.md`: industrial world
  command table direction, stage-first layout, diagnostics demotion, and
  `World / Targets / Command` mobile route.
- `viewer-pixel-world-semantic-positioning-2026-05-26.prd.md`: sparse snapshot
  semantic positioning, `location_derived` honesty, and relationship-line
  readability.
- `viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md`:
  commercial HUD, player leverage, route readability, and diagnostics collapse.
- `doc/product/agents-world-simulation/player-readable-world-stage.design.md`:
  durable object, relationship, scale-honesty, and diagnostic-demotion product
  semantics distilled from the retired 2D/EGUI readability topics.
- `doc/testing/manual/model-visual-review-sop-2026-05-29.manual.md`:
  screenshot-based visual review gate, model review rubric, and escalation
  boundaries.

Runtime, QA, release, LiveOps, and GitHub governance authority remains with the
corresponding role/process. This document defines visual direction and review
expectations.

## 4. Product Visual North Star
The Viewer must feel like an industrial world command table, not a generic dark
diagnostics dashboard.

The player should read the screen in this order:
1. What world or stage am I looking at?
2. Which agent, location, objective, blocker, or route matters now?
3. What can I do next?
4. What happened because of the latest player or agent action?
5. Where can I open deeper diagnostics if I need them?

This order is more important than any single color, card shape, or layout
variant. When space is constrained, preserve the reading order and remove
secondary detail before shrinking the main subject.

## 5. Non-Negotiable Principles
### 5.1 World First
- The world stage is the primary visual surface in Player mode.
- Diagnostics, raw JSON, renderer badges, provider checks, and governance labels
  must never compete with the world stage on first load.
- If a renderer is degraded or missing, the unavailable diagnostic must show the
  honest reason without constructing a second JS world.

### 5.2 Player Leverage Before Ambient Activity
- The UI must distinguish player-caused progress from ambient simulation
  movement.
- A busy world is not automatically a successful player action.
- Receipts must answer: what was attempted, what changed, what is blocked, and
  what the next step is.

### 5.3 Diagnostics Are Available, Not Dominant
- Director and QA capabilities remain reachable.
- In Player mode, advanced controls and diagnostics are demoted to drawers,
  collapsible panels, secondary tabs, or explicit mode switches.
- Hiding diagnostics is acceptable only when the route back to them remains
  discoverable and scriptable.

### 5.4 Data Honesty
- Derived visual positions, unavailable states, inferred routes, and missing
  runtime fields must be labelled or visually scoped so they cannot be mistaken
  for authoritative runtime truth.
- Visual hierarchy must not imply progress, causality, health, or ownership
  that the DTO/state does not support.

### 5.5 One Obvious Command Path
- Player mode must keep the primary command route one explicit step away from
  the world stage.
- Advanced layouts, debug presets, and secondary prompt tools may exist, but
  they must not hide the first command path.
- If the primary command is disabled, blocked, or waiting on data, the reason
  and recovery path must be visible.

### 5.6 Sparse Worlds Still Communicate Relationships
- Sparse snapshots must still read as a world, not as isolated dots.
- Agent-location links, route lines, hotspots, semantic badges, and textual
  summaries are mandatory tools for sparse-state readability.
- Fallback DOM views should preserve relationships whenever canvas/wasm surfaces
  are degraded.

### 5.7 Readability Beats Decorative Density
- Avoid adding decorative panels, badges, grids, particles, or status chips when
  they reduce scan speed.
- Fragment terrain, location anchors, and background world texture support the
  active agent/action read; they do not become the first visual subject unless a
  task specifically makes them the subject.

## 6. Visual Language
The durable brand-system companion for this section is
`viewer-brand-system-2026-06-05.design.md`. New visible Viewer work must use the
token taxonomy, icon/status vocabulary, asset-language rules, and screenshot
matrix defined there.

### 6.1 Tone
- Industrial, precise, command-oriented, and playable.
- The interface can feel technical, but it should not feel like an internal log
  viewer by default.
- Avoid one-note dark dashboard language where every surface has the same weight.

### 6.2 Surface Hierarchy
Use four surface levels:

| Level | Purpose | Examples | Default prominence |
| --- | --- | --- | --- |
| Stage | Primary world comprehension | Pixel-world canvas, 2D map, selected target overlay | Highest |
| Command | Player decision and action | objective, next action, chat command, action receipt | High |
| Context | Helpful state and explanation | target details, recent events, route counts, lightweight summaries | Medium |
| Diagnostics | Engineering and QA visibility | renderer status, raw DTO, provider checks, governance lanes | Low / collapsed |

### 6.3 Color Semantics
Color is semantic, not ornamental:
- `world-background`: low-distraction stage base.
- `terrain-subdued`: background Fragment and world-body detail.
- `agent-primary`: player-relevant active actor.
- `route-active`: current or selected relationship/action path.
- `objective-accent`: current goal and recommended next step.
- `blocker-danger`: blocked/error/missing requirement.
- `receipt-success`: applied/completed feedback.
- `diagnostic-muted`: technical or QA-only metadata.
- `panel-surface`: command/context tool surfaces.
- `focus-ring`: keyboard and selection focus.

Do not encode critical state only by color; pair color with label, icon, shape,
or placement.

### 6.4 Typography and Copy Density
- `world-title`: stage identity.
- `objective-title`: current goal.
- `action-label`: executable or recommended next step.
- `receipt-body`: result feedback and cause/effect text.
- `diagnostic-caption`: engineering/status metadata.

- Player-facing headings should be short and action-oriented.
- Compact panels should use compact headings; reserve large type for true stage
  or route-level focus.
- Diagnostic labels may be terse, but player receipts and blockers must be
  understandable without reading raw state names.
- CJK text must remain readable and must not fallback to missing glyph boxes.

### 6.5 Shape and Layout
- Use strong alignment, consistent spacing, and clear grouping instead of nested
  card walls.
- Cards are for repeated items, modal-like surfaces, or true framed tools, not
  for wrapping every page section.
- Stable stage, toolbar, grid, and panel dimensions are required so hover,
  status text, labels, and loading states do not shift the layout.

### 6.6 Status Vocabulary
The following states need stable visual and text equivalents:
- selected;
- target;
- blocker;
- active route;
- accepted intent;
- executing;
- completed;
- no receipt;
- derived position;
- unavailable or degraded renderer.

## 7. Information Architecture
### 7.1 Player Mode Default
Player mode must show:
- world/stage;
- current objective or selected target;
- next action or blocker;
- latest receipt or current action feedback;
- a clear route into command/chat.

Player mode must demote:
- raw runtime JSON;
- renderer implementation status;
- provider/auth/governance detail;
- test controls;
- long event lists.

### 7.2 Director Mode
Director mode may expose denser diagnostics, but it still follows data honesty
and hierarchy rules. Director mode is not permission to make every surface equal.

### 7.3 HUD and Side Panel Hierarchy
- HUD: objective, next action, player leverage, blocker, receipt.
- Right/side panel: selected target, command/chat, recent context, then
  diagnostics.
- Diagnostics panel: runtime status, renderer status, provider/auth/governance,
  raw DTO, test controls.

### 7.4 Empty, Loading, Error, and Fallback Hierarchy
- First: state name and whether player action is possible.
- Second: reason or missing dependency.
- Third: recovery path or next observable signal.
- Fourth: diagnostic detail, collapsed unless it is the recovery path.

### 7.5 Mobile and Narrow Screens
Mobile must preserve the same conceptual route as desktop:
- `World`
- `Targets`
- `Command`
- optional `Diagnostics`

Do not rely on one long single-column stack when that makes the player scroll
past the world before finding the command path.

## 8. Pixel-World Layering
Pixel-world surfaces use this priority order:
1. World Command Board: objective, next action, player leverage.
2. Action Receipt / Blocker: latest meaningful feedback.
3. Active Agent and selected target.
4. Routes, relationships, hotspots, and goal callouts.
5. Locations and logical anchors.
6. Fragment terrain and ambient background.
7. Renderer diagnostics and raw DTO controls.

Fragment terrain should make the world feel physically present, but the active
agent, route, objective, and receipt must remain easier to identify.

When exact coordinates are missing:
- derived positions may be used for readability;
- the source must remain inspectable as `location_derived`, `missing`, or
  equivalent;
- derived visuals must not claim runtime authority.

## 9. 2D Map Rules
- 2D mode may introduce high-contrast Location and Agent symbols that are not
  shown in 3D mode.
- Label LOD must prioritize selected/active Agent, current objective, blocker,
  and route labels before ambient Location labels.
- Flow overlays should use direction arrows or equivalent marks when they
  clarify current movement, logistics, or command effect.
- 2D overlays must not leak into 3D mode after camera mode changes.
- Dense map labels should degrade by hiding secondary labels, not by shrinking
  all text below readability.

## 10. Interaction and Feedback
### 10.1 Selection
- Selecting an entity must produce a visible focus state and a readable details
  surface.
- Selection should not hide the current objective or next step.
- If an entity is decorative/background-only, do not make it look selectable.

### 10.2 Command
- The command route must be visible within one player-facing step from the main
  stage.
- Chat/prompt controls must not be buried behind diagnostics.
- Disabled or unavailable commands require a clear reason.

### 10.3 Receipt
Every action feedback surface should answer:
- action: what was attempted;
- result: what changed or did not change;
- reason: why it succeeded, stalled, or failed;
- next: what the player can do now.

### 10.4 Loading, Empty, Error, and Fallback States
- Loading states must reserve layout space and avoid stage jumps.
- Empty states must explain whether the world is empty, still loading, filtered,
  or degraded.
- Error states must distinguish player-fixable blockers from system failures.
- Fallback rendering must be explicit; silent renderer substitution is not a
  valid player-facing state.

## 11. Accessibility and Robustness
- Critical text must remain readable on desktop and mobile viewports.
- Primary controls must be keyboard reachable where the surface supports DOM
  interaction.
- Focus states must be visible.
- Color contrast must be sufficient for objective, blocker, receipt, and command
  labels.
- Motion and animation should clarify cause/effect; do not depend on motion as
  the only indicator of progress.
- Avoid horizontal overflow, clipped button text, overlapping labels, and hidden
  main actions.

## 12. Visual Review Gate
Any change that affects visible output must trigger screenshot-based visual
review under `doc/testing/manual/model-visual-review-sop-2026-05-29.manual.md`.

Minimum evidence:
- desktop screenshot of the target surface;
- mobile screenshot when the change touches first screen, responsive behavior,
  navigation, stage layout, command route, or text density;
- automated summary such as DOM/UI tests, console/state probe, visual smoke, or
  explicit reason when an automated check is not available;
- expected visual contract for the change.

For visual-system or large surface changes, also apply the expanded matrix in
`viewer-brand-system-2026-06-05.design.md#6-screenshot-and-visual-evidence-matrix`.

Must-pass checks:
- first visual focus matches the task goal;
- stage, objective, command, receipt, and diagnostics are ordered correctly;
- no blocking overlap, clipping, horizontal overflow, or illegible text;
- UI does not imply unsupported causality or progress;
- mobile preserves the same main route as desktop;
- diagnostics do not dominate Player mode.

Escalate to human or owner review when:
- visual direction, interaction feel, or player screen flow is disputed;
- model confidence is low;
- screenshot evidence lacks a key viewport;
- the verdict affects release notes, public claims, or user promises;
- the same issue remains `watch` after two rounds.

Pixel-world specific blockers:
- Agent is less visible than Fragment terrain.
- Renderer diagnostics are louder than the command board or action receipt.
- Location markers dominate as main entities when the task is agent/action
  focused.
- Derived or missing positions lack an honest source marker.
- Sparse snapshots do not show relationships, route, or readable world state.

## 13. Implementation Brief for Viewer Work
Viewer implementation tasks that change visible surfaces should include:
- target mode: Player, Director, or both;
- primary first-read subject;
- affected stage/command/context/diagnostics level;
- expected receipt/blocker/empty/unavailable behavior;
- desktop and mobile screenshot plan;
- DOM or semantic test anchors that must remain stable;
- explicit non-goals, especially protocol, runtime, and art-pipeline boundaries.

## 14. Acceptance Criteria for Future Visual Changes
- The change preserves world-first reading order.
- Player leverage is more prominent than ambient simulation activity.
- Diagnostics remain reachable but demoted.
- Visual states map honestly to current DTO/runtime truth.
- Pixel-world layers follow the priority order in this spec.
- Responsive layouts preserve `World / Targets / Command`.
- Screenshot review and automated evidence are attached before completion claims.

## 15. Governance
- This document is the canonical visual design specification for Viewer and
  player-facing world-simulator surfaces.
- Topic-specific PRDs may add stricter rules for their own scope, but they should
  not weaken this document's hierarchy, data honesty, or review-gate rules.
- If a future task changes the default visual direction, update this document and
  link the decision from `doc/world-simulator/viewer/README.md`.

## 16. Residual Risks
- The final Viewer brand book, token taxonomy, icon/status vocabulary,
  asset-language rules, and expanded screenshot matrix now live in
  `viewer-brand-system-2026-06-05.design.md`.
- The brand-system companion does not create a new runtime feature, public
  marketing identity, or external art-production pipeline.
- It does not replace `viewer_engineer` feasibility judgment, `qa_engineer`
  release-blocking judgment, or `liveops_community` public messaging ownership.
