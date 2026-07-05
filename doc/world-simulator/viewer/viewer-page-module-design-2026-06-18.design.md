# Viewer Page and Module Design (2026-06-18)

- Professional owner: `game_visual_interaction_designer`
- Integration owner: `tpm`
- Source task uid: `task_e7760ad76a0046dfa5a17d0a5a89e59c` / GitHub issue #1496; execution evidence is in GitHub task issue evidence comments and `.pm/github-project-sync/task-archive.jsonl`.
- Slice type: `page_overall_and_module_design`
- Companion specs:
  - `doc/world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md`
  - `doc/world-simulator/viewer/viewer-brand-system-2026-06-05.design.md`
- Image2 visual target:
  - `output/visual-targets/viewer-visual-polish/image2-viewer-first-screen-target.png`

## 1. Purpose
This document converts the current complex Viewer page into an executable
page-level and module-level visual interaction design. It does not replace the
June 5 visual and brand-system specifications. It narrows them to the current
`software_safe` Viewer page so implementation can proceed in safe slices.

The page is a player-facing command table. Its job is to let a player read the
world, understand the active objective, find the next command route, see the
latest action receipt, and reach details or diagnostics without the first screen
becoming a diagnostics dashboard.

This design owns visual hierarchy, interaction readability, module rhythm,
state expression, and screenshot-review expectations. It does not own runtime
correctness, engineering feasibility, QA release judgment, product priority, or
new gameplay semantics.

## 2. Overall Page Design
### 2.1 Screen Purpose
The Viewer first screen must answer, in order:

1. What world or stage am I looking at?
2. Which objective, actor, blocker, or route matters now?
3. What can I do next?
4. What happened because of the latest action or intent?
5. Where can I inspect targets, command details, gameplay state, or diagnostics?

The page should feel like an industrial world command table, not a landing page
and not an internal observability console.

### 2.2 Information Architecture
Use four priority levels:

| Level | Viewer surface | Purpose | First-screen rule |
| --- | --- | --- | --- |
| Stage | `#viewer-stage-panel`, `.pixel-world-canvas` | world comprehension | largest visual subject |
| Command | `.pixel-world-command-strip`, `.pixel-world-action-receipt`, `#viewer-details-panel` | decision and feedback | visible within one step from stage |
| Context | `#viewer-targets-panel`, `#viewer-gameplay-details`, selected target details | selection, explanation, state machine | useful, not louder than stage |
| Diagnostics | `#viewer-diagnostics-panel`, renderer details, raw JSON | debug and QA visibility | collapsed or visually muted by default |

Desktop layout keeps the current three-zone structure:

- left: Targets and selection context;
- center: Stage, command strip, world board, receipt, gameplay details;
- right: Details and command surface;
- diagnostics: collapsed inside the details/diagnostics route.

The center stage owns the first visual read. Left and right panels should frame
the command table without stealing the first glance.

### 2.3 Reading Order
Desktop first viewport:

1. Compact stage identity and current objective.
2. Command strip with Objective, Next Move, and Player Leverage.
3. Pixel World Board/Canvas.
4. Action Receipt.
5. Targets and Details panels as side routes.
6. Gameplay Details and Diagnostics as disclosure surfaces.

Mobile reading order:

1. Mobile rail: `World / Targets / Command`, with Diagnostics secondary.
2. Compact stage identity.
3. Command strip in one column.
4. World Board/Canvas.
5. Action Receipt.
6. Targets, Command, Gameplay Details, Diagnostics through anchors or tabs.

### 2.4 Layout Zones
The page should behave as a command-table shell:

- Header identity: compact, task-aware, no large marketing hero.
- Command band: objective, next move, player leverage.
- World board: the main visual canvas and route/selection read.
- Feedback band: action receipt, blocker, no-receipt, or unavailable state.
- Side context: target list and command panel.
- Deep context: gameplay details, auth/economy, diagnostics.

The command band and receipt may bracket the board, but they must not compress
the board into a secondary widget.

### 2.5 Responsive Route
Desktop can use three columns. Tablet should keep the stage first, then targets
and command as stacked panels. Mobile must not rely on a long undifferentiated
page stack. Preserve explicit `World / Targets / Command` navigation and keep
Diagnostics visible but lower priority.

## 3. Module Design
### 3.1 Stage Hero
Current anchors: `WorldStageHero`, `.stage-hero`, `.hero-focus-grid`.

Role:
- establishes screen identity and current situation;
- exposes connection warnings only when they require attention;
- provides quick routes to targets and command on narrow screens.

Design contract:
- keep it compact when `PixelWorldHost` has a commercial surface;
- show one stage identity and one current goal line;
- avoid repeating the full Objective, Next Move, and receipt already handled by
  Pixel World Board modules;
- connection warning may be visually elevated only when disconnected,
  connecting, closed, or otherwise abnormal;
- selected-target badges may appear, but they remain secondary to the board.

State handling:
- loading: reserve height and show a short "syncing world" style line;
- empty/no snapshot: explain that the world is waiting for canonical state;
- blocked/connection issue: elevate the warning and recovery route;
- long text or zh locale: title wraps to two lines before pushing the board
  below the first viewport.

### 3.2 Pixel World Board / Canvas
Current anchors: `PixelWorldHost`, `.pixel-world-canvas`,
`.pixel-world-canvas--rendered`, `.pixel-world-canvas__surface`.

Role:
- primary visual subject;
- shows world, active agent, selected target, route, objective callout, blocker,
  and unavailable-state honesty;
- gives the player spatial confidence before command or diagnostics detail.

Design contract:
- occupies the largest center-zone area on desktop;
- preserves a stable aspect ratio and minimum height across loading, unavailable,
  rendered, and selected states;
- active agent, selected target, objective callout, and route must read above
  fragments, terrain, and ambient labels;
- renderer unavailable is valid only as an explicit diagnostic state;
- canvas controls or renderer metadata must never look like the primary action.

State handling:
- loading: skeleton or reserved board frame with stage label;
- unavailable: clear renderer unavailable state plus honest reason;
- blocked: blocker callout near the route or target, paired with receipt;
- selected: selected entity gets a visible focus callout and details route;
- long labels/zh: prefer label truncation with details route over shrinking
  the board text below readability.

### 3.3 Command Strip
Current anchors: `.pixel-world-command-strip`,
`.pixel-world-command-cell--objective`, `.pixel-world-command-cell--next`,
`.pixel-world-command-cell--leverage`.

Role:
- one-row command summary above the board on desktop;
- one-column pre-board summary on mobile;
- tells the player current objective, next move, and why player action matters.

Design contract:
- Objective, Next Move, and Player Leverage are peers, but Next Move receives
  the strongest action affordance;
- `Go to Command` may be used as a conservative route when direct execution has
  not been verified by `viewer_engineer`;
- do not imply a direct runtime action unless an existing published action is
  safely bound;
- blocked or unavailable next move must show the reason in the same cell.

State handling:
- empty/no commercial surface: strip should not render or should collapse into
  a clear waiting state;
- blocked: Next Move cell changes from action route to blocker/recovery route;
- completed: Next Move may point to next objective or review receipt;
- long text/zh: cells wrap within fixed tracks; action control remains visible.

### 3.4 Action Receipt
Current anchor: `.pixel-world-action-receipt`.

Role:
- confirms player-caused or accepted-intent feedback;
- distinguishes no receipt, accepted intent, world delta, blocked, rejected, and
  completed states;
- prevents ambient simulation motion from being misread as player progress.

Design contract:
- visible near the board and command strip, before diagnostics;
- title, summary, detail, and confidence meta are all visually distinct;
- no-receipt is a real state, not an absence of UI;
- blocked/rejected states use warning or danger language plus recovery route;
- completed states can be positive but must not imply unsupported causality.

State handling:
- no receipt: muted but explicit;
- blocked/rejected: visually higher than diagnostic metadata;
- completed: success state with concise summary;
- long detail: clamp or wrap with a details affordance, not layout overflow.

### 3.5 Targets Panel
Current anchor: `#viewer-targets-panel`.

Role:
- selection and orientation surface;
- lets the player choose the agent, location, or target that the stage and
  command panel will follow.

Design contract:
- target rows should read as selectable controls, not diagnostics;
- current selection must be mirrored in the stage or board;
- pending/syncing states reserve list space;
- raw counts are secondary to selectable names and state badges.

State handling:
- loading: pending row skeleton;
- empty: explain whether there is no snapshot, no targets, or a filter result;
- selected: visible selected state and anchor to stage/command;
- zh/long names: preserve row controls and avoid horizontal overflow.

### 3.6 Details / Command Panel
Current anchor: `#viewer-details-panel`.

Role:
- command execution route and selected-target details;
- right-side command area on desktop;
- target of `Go to Command` on mobile and conservative Next Move routing.

Design contract:
- primary command or chat route appears before raw prompt and inspection tools;
- disabled command states must show reason and recovery;
- selected-target context appears before raw JSON;
- if Next Move routes here, the destination section should visibly match the
  action label enough that the player knows they arrived correctly.

Handoff:
- direct Next Move execution feasibility belongs to `viewer_engineer`;
- gameplay meaning of new action states belongs to `gameplay_designer` and
  `runtime_engineer`.

### 3.7 Gameplay Details
Current anchor: `#viewer-gameplay-details`.

Role:
- expanded state-machine, economy, auth, and formal gameplay details;
- context layer, not first-screen command layer.

Design contract:
- default collapsed after this polish pass unless the page lacks summary data;
- summary line explains why it exists: full state machine and economy detail;
- never duplicates the command strip at equal visual weight;
- contains formal status, accepted intent, execution state, economy, auth, and
  hosted surfaces without re-promoting diagnostics.

State handling:
- empty/no canonical gameplay snapshot: concise waiting explanation;
- blocked: formal blocker detail lives here, but primary blocker remains near
  command/receipt;
- completed: can show history and state machine, not replace receipt.

### 3.8 Diagnostics
Current anchors: `#viewer-diagnostics-panel`, `.diagnostic-surface`,
`.pixel-world-render-diagnostics`, `.pixel-world-render-unavailable`.

Role:
- engineering and QA visibility;
- renderer, provider, auth, raw DTO, and bridge details;
- reachable, scriptable, and honest, but visually quiet.

Design contract:
- collapsed by default in Player mode;
- renderer unavailable is visible when it affects the world board, but diagnostic
  detail stays in a secondary disclosure;
- raw JSON never appears before command, board, receipt, or target selection;
- badges use diagnostic-muted weight unless warning or failure affects player
  recovery.

State handling:
- unavailable: explicit label, reason, and route to details;
- fatal renderer: warning badge plus diagnostic expansion;
- no runtime/provider: distinguish player-fixable from system issue;
- long diagnostic text: scroll or collapse inside diagnostic surface.

### 3.9 Focus Mode
Current anchors: `.pixel-world-host--focus`, `.pixel-world-focus-hud`,
`.pixel-world-focus-cinematic`, `.pixel-world-focus-drawer`.

Role:
- immersive world-first mode for reading objective, route, blocker, receipt,
  and issuing command without the full shell.

Design contract:
- focus HUD uses stable command vocabulary: World Focus, Current Objective,
  Mission Progress, World Tick, Blocker, Receipt;
- board remains the main subject;
- command drawer is a clear route, not a hidden debug panel;
- diagnostics drawer remains separate and lower priority;
- exit/maximize controls are visible and keyboard focusable.

State handling:
- renderer unavailable: minimap is absent and the diagnostic is honest and compact;
- no selected agent: command surface explains selection requirement;
- blocked: blocker cell rises above receipt detail;
- mobile: focus HUD stacks without overlapping the board or controls.

### 3.10 Mobile Route
Current anchor: `.mobile-rail`.

Role:
- preserves the same IA without forcing the player to scroll blindly.

Design contract:
- primary route is `World / Targets / Command`;
- Diagnostics can exist as a quieter fourth link;
- first mobile viewport should include stage identity plus either command strip
  or board start;
- avoid horizontal overflow from badges, long labels, or command controls;
- buttons and links need stable tap targets.

State handling:
- long zh labels: wrap or shorten nav labels, do not overflow;
- blocked/offline: show warning near world, not only inside diagnostics;
- focus mode: hide shell panels but keep exit and command controls reachable.

## 4. State Model
All modules should map to these player-facing states:

| State | Visual requirement | Notes |
| --- | --- | --- |
| empty | explicit empty reason and next observable signal | distinguish empty world, no target, no receipt |
| loading | reserved layout, `aria-busy` or equivalent where DOM supports it | no stage jump |
| unavailable | clear renderer unavailable label near affected board | raw details stay secondary |
| blocked | blocker reason near next move and receipt | recovery route visible |
| no receipt | muted receipt surface remains present | prevents false progress |
| completed | positive receipt state plus next route | do not imply unsupported runtime causality |
| selected | visible focus state in list, board, and details | selection must not hide objective |
| long text | wrap or clamp within stable modules | no horizontal overflow |
| zh locale | CJK labels fit primary controls and panels | screenshot required when touched |

## 5. Image2 Target Usage
The Image2 target is binding as a direction for:

- world board as the dominant first-screen subject;
- compact Objective / Next Move / Player Leverage command band;
- visible but concise Action Receipt;
- diagnostics and auxiliary panels demoted to side or disclosure areas;
- industrial command-table tone and clear spatial route reading.

The Image2 target is aspirational, not binding, for:

- exact map art, terrain density, icons, and pixel illustration style;
- exact browser chrome, brand mark, right-side accordion labels, or left-side
  resource panel;
- exact colors beyond the existing Viewer token taxonomy;
- exact layout measurements.

The Image2 target cannot infer:

- runtime DTO availability, action execution feasibility, or renderer status;
- DOM hooks, tests, focus behavior, keyboard accessibility, or screen-reader
  labels;
- mobile stacking behavior;
- zh locale length and CJK typography;
- empty, loading, no-receipt, unavailable, blocked, completed, and long-text
  states.

Use the target by comparing real desktop and mobile Viewer screenshots against
the binding direction above. Record gap notes before claiming the visual pass
has converged.

## 6. Implementation Slicing Recommendation
### Slice 1: Page IA and duplication control
Goal: make the first screen read as one command table.

Recommended work:
- keep `WorldStageHero` compact;
- keep `WorldSummaryPanel` as collapsed `Gameplay Details`;
- preserve stable anchors:
  - `#viewer-stage-panel`
  - `#viewer-targets-panel`
  - `#viewer-details-panel`
  - `#viewer-diagnostics-panel`
  - `#viewer-gameplay-details`

Verification gate:
- UI tests for stable anchors and copy changes;
- desktop screenshot confirms no three equal objective summaries.

### Slice 2: Command strip and receipt hierarchy
Goal: make Objective, Next Move, Player Leverage, and Action Receipt the
command band around the board.

Recommended work:
- strengthen Next Move as route to command, not direct action unless verified;
- ensure blocked/no-receipt/completed variants have clear visual states;
- quiet readout badges.

Verification gate:
- `test:ui`;
- receipt/no-receipt assertions;
- desktop and mobile screenshot comparison with Image2 target direction.

### Slice 3: Board prominence and unavailable honesty
Goal: make the canvas board the main subject across renderer states.

Recommended work:
- enforce stable board dimensions;
- keep renderer unavailable explicit;
- ensure selected/route/objective/blocker layers read above terrain.

Verification gate:
- `build:software-safe`;
- `test:pixel-world:visual`;
- external browser runtime check if renderer/WebGPU behavior conflicts with
  in-app browser evidence.

### Slice 4: Focus mode and mobile route
Goal: keep immersive mode and mobile route coherent after hierarchy changes.

Recommended work:
- verify focus HUD labels, command drawer, diagnostics drawer, and exit controls;
- verify mobile `World / Targets / Command` route, long labels, and no overlap.

Verification gate:
- focus-mode visual screenshot;
- mobile screenshot at 390x844 or equivalent;
- zh locale screenshot or overflow probe.

## 7. Verification Gates
Recommended commands when the local Node environment is available:

- `npm --prefix crates/oasis7_viewer run test:ui`
- `npm --prefix crates/oasis7_viewer run build:software-safe`
- `npm --prefix crates/oasis7_viewer run test:pixel-world:visual`

Recommended screenshot scenarios:

- desktop first screen, `locale=en`;
- desktop first screen, `locale=zh`;
- mobile first screen, `locale=en`;
- mobile first screen, `locale=zh`;
- renderer unavailable;
- action receipt present;
- no receipt;
- blocked next move;
- completed receipt;
- focus mode, desktop and mobile.

## 8. Residual Risks and Role Handoffs
- `viewer_engineer`: decide whether Next Move can directly execute a published
  action or should remain an anchor to `#viewer-details-panel`; validate DOM,
  renderer, responsive, and implementation feasibility.
- `qa_engineer`: own pass/fail release judgment after tests, screenshots, and
  browser smoke evidence.
- `runtime_engineer`: required if UI changes need new receipt, blocker, world
  delta, or renderer status semantics.
- `gameplay_designer`: required if Next Move, Player Leverage, or objective copy
  changes gameplay meaning, player verbs, or progression expectations.
- `producer_system_designer`: required if this changes product priority,
  Player/Director defaults, or scope boundaries.
- `liveops_community`: not required for this design unless later work drafts
  public messaging, release notes, player promises, or community explanations.

## 9. Professional Slice Verdict
Proceed with the visual polish only as a page-architecture and module-contract
pass, then implement in small slices. The existing provisional implementation is
directionally aligned with the compact-hero, collapsed-gameplay-details, command
proximity, and diagnostics-demotion goals, but it still needs real screenshot
gap notes against the Image2 target and module-state verification before any
completion claim.
