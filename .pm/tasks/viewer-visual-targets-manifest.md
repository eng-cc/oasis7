# Viewer Visual Targets Manifest

- task_uid: `task_060e9de147ba4757ac29cf0fb7a15210`
- role: `game_visual_interaction_designer`
- scope: effect-image-first planning for the `software_safe.html` viewer SPA
- source basis: `software_safe.html`, `software_safe_src/main.jsx`, `software_safe_src/pixel_world_host.jsx`, current `.pm/tasks/viewer-*.png` references

## Inventory

The viewer is one SPA, not a multi-route site. The target set should therefore cover user-facing surfaces and states:

1. Desktop command desk shell: targets rail, stage/world board, command column.
2. Mobile command desk shell: stacked mobile surfaces with jump rail.
3. Desktop World Focus selected-blocker state: selected agent, blocker, receipt, route/fragments, command drawer.
4. Mobile World Focus selected-blocker state: same state with readable map band and bottom command drawer.
5. Agent command/chat state: selected agent command surface, chat draft, message flow, prompt controls collapsed.
6. Gameplay details + diagnostics state: formal gameplay details expanded, runtime diagnostics visible but subordinate.
7. Hosted login/access gate state: modal/gate visible when hosted public join requires player session.
8. Empty-world recovery state: no entities or pending snapshot, with recovery guidance and disabled/limited command affordance.

## Target Images

### 1. Desktop Command Desk Shell

- Recommended output: `.pm/tasks/viewer-target-layout-desktop.png`
- Viewport: 1440 x 1000
- Existing references: `.pm/tasks/viewer-layout-desktop.png`
- Prompt:

```text
Create a high-fidelity game UI target image for the oasis7 software-safe viewer desktop command desk. 1440x1000. Dark industrial sci-fi operations interface, not a marketing landing page. One SPA view with three columns: left "Targets" navigation rail with filter search, selected agent and location lists; center "Industrial World Command Desk" stage with compact hero, world command board, map-like pixel world surface, objective, next step, action receipt and gameplay summary; right "Interact and Inspect" command column focused on the selected agent. Preserve real oasis7 concepts only: agents, locations, world tick, routes, fragments, blocker, action receipt, gameplay details, diagnostics. Use restrained dark green/charcoal background, warm amber objective accents, mint agent accents, red-orange blocker accents. Dense but readable operational layout, 8px or similarly restrained radii, crisp borders, no fantasy characters, no impossible blockchain/token-transfer promises, no fake charts unrelated to the current viewer. Text should be short UI labels, not promotional copy. The player should immediately read: current target, world state, next action, why blocked, and where to command.
```

- Acceptance criteria:
  - The first viewport reads as a workbench, not a landing page.
  - Left/center/right hierarchy is clear at a glance.
  - Stage/world board is visually dominant without hiding target and command surfaces.
  - No invented product capability beyond current viewer surfaces.
- Implementation order: first, because it defines the global shell and token system.
- Fixture/test needs:
  - `test_api=1` fixture with selected agent, at least one location, route, blocker, receipt, and chat capability status.
  - DOM markers already present: `data-viewer-surface="targets|stage|command"`.

### 2. Mobile Command Desk Shell

- Recommended output: `.pm/tasks/viewer-target-layout-mobile.png`
- Viewport: 390 x 844
- Existing references: `.pm/tasks/viewer-layout-mobile.png`
- Prompt:

```text
Create a high-fidelity mobile target image for the oasis7 software-safe viewer command desk. 390x844. Single-page stacked game operations UI with compact sticky jump rail for World, Targets, Command, Diagnostics. The visible first screen prioritizes the world stage and current objective, with targets and command surfaces reachable without looking like a desktop squeezed onto mobile. Dark industrial palette with mint agent accents, amber objective accents, red-orange blocker accents. Use real viewer surfaces only: Targets, World Command Board, selected agent, command/chat, gameplay details, runtime diagnostics. The layout should keep touch targets large, avoid text overlap, keep dense panels scannable, and show that mobile is a designed mode rather than a collapsed afterthought. No fictional characters, no decorative orbs, no fake token transfer UI.
```

- Acceptance criteria:
  - No horizontal overflow at 390px.
  - Mobile rail is useful but not visually louder than the world/action state.
  - The first screen gives a clear next step before long diagnostic content.
- Implementation order: second, paired with desktop shell tokens before focus states.
- Fixture/test needs:
  - Same selected-agent shell fixture as desktop.
  - Screenshot probe for `document.body.scrollWidth <= window.innerWidth`.

### 3. Desktop World Focus Selected Blocker

- Recommended output: `.pm/tasks/viewer-target-focus-selected-blocker-desktop.png`
- Viewport: 1440 x 960
- Existing references: `.pm/tasks/viewer-focus-desktop.png`, `.pm/tasks/viewer-focus-selected-blocker-desktop-final-2026-06-18.png`
- Prompt:

```text
Create a high-fidelity desktop target image for oasis7 "World Focus" mode. 1440x960 full-screen game viewer, selected agent agent-0 on a pixel-world industrial map with fragment terrain patches, route line to Factory Anchor, and a visible selected-agent marker. HUD-first composition: top mission HUD with current objective "Recover sustainable capability", progress around 68%, blocker "Missing Material", world tick 12, and action receipt "Action blocked". Left focus rail lists World Focus, Agent, Routes, Fragments without crowding the HUD. Bottom/side command drawer is open and shows the selected agent command/chat surface. No onboarding cinematic banner in this rich selected-agent state. Dark map-first background, mint route/agent accents, amber mission accent, red-orange blocker/receipt emphasis. The player should read in two seconds: who is selected, why action is blocked, what changed, and what command surface is available.
```

- Acceptance criteria:
  - `agent/agent-0`, blocker, receipt, route/fragments, and command drawer are all visible.
  - No `[data-focus-cinematic="true"]` in comparable rich state.
  - Focus controls do not compete with the mission/blocker/receipt path.
  - Left rail and world tick do not overlap or form a cramped stack.
- Implementation order: third, after shell tokens.
- Fixture/test needs:
  - Existing fixture: `pixel_world_visual_fixture=selected_blocker`.
  - Existing markers: `data-visual-fixture="selected_blocker"`, `data-focus-comparable="true"`.

### 4. Mobile World Focus Selected Blocker

- Recommended output: `.pm/tasks/viewer-target-focus-selected-blocker-mobile.png`
- Viewport: 390 x 844
- Existing references: `.pm/tasks/viewer-focus-mobile.png`, `.pm/tasks/viewer-focus-mobile-drawer.png`, `.pm/tasks/viewer-focus-selected-blocker-mobile-final-2026-06-18.png`
- Prompt:

```text
Create a high-fidelity mobile target image for oasis7 World Focus selected-blocker state. 390x844. Full-screen mobile game UI with selected agent agent-0 on a compact pixel-world map, visible mission HUD, blocker "Missing Material", receipt "Action blocked", world tick 12, and a bottom command drawer that is open but does not blanket the map. The map band remains meaningful above the drawer; the command sheet shows selected target and chat/command context. Use dark industrial surfaces, mint agent/route accents, amber objective, red-orange blocker. Touch controls are icon-like and compact, text fits without wrapping awkwardly, no horizontal overflow, no cinematic banner in this rich selected-agent state.
```

- Acceptance criteria:
  - Map remains visibly readable above the drawer.
  - Tick, blocker, receipt, and selected target fit without digit/word wrapping.
  - Command drawer cap remains below half viewport unless explicitly expanded.
- Implementation order: fourth, after desktop focus.
- Fixture/test needs:
  - Existing selected-blocker fixture.
  - Screenshot probe for overflow and drawer height ratio.

### 5. Agent Command / Chat

- Recommended output: `.pm/tasks/viewer-target-command-agent-chat.png`
- Viewport: 1440 x 1000 desktop, optional 390 x 844 crop later if implementation risk appears
- Existing reference: right column areas in `.pm/tasks/viewer-layout-desktop.png`
- Prompt:

```text
Create a high-fidelity target image for the oasis7 selected-agent command surface inside the software-safe viewer. Desktop operational UI, focused on the right "Interact and Inspect" column while the left targets and center world stage remain visible. Selected target is agent=agent-0. Show Agent Chat as the primary command surface with a message textarea, send button, chat status badge, recent message flow, and prompt controls collapsed below as advanced controls. Include capability badges such as chat ready/limited, prompt status, bound player/key placeholders only as existing viewer status labels. Asset/governance lane is present at the bottom as disabled or boundary guidance, not a live transfer form. The design should feel like a game command terminal: readable, restrained, decisive, with no fake wallet transfer screen or invented automation claim.
```

- Acceptance criteria:
  - Chat/command is primary; prompt overrides and asset/governance are visually subordinate.
  - Disabled/limited capabilities are clear without looking broken.
  - Selected target context remains visible throughout.
- Implementation order: fifth, after map/focus states.
- Fixture/test needs:
  - Need `test_api` fixture for selected agent with chat history, chat capability ready/limited variants, and prompt overrides collapsed.
  - Need DOM marker or screenshot query for `data-chat-send`, selected `agent=agent-0`, and prompt override collapsed state.

### 6. Gameplay Details + Diagnostics

- Recommended output: `.pm/tasks/viewer-target-gameplay-diagnostics.png`
- Viewport: 1440 x 1200
- Existing reference: lower sections in `.pm/tasks/viewer-layout-desktop.png`
- Prompt:

```text
Create a high-fidelity target image for the oasis7 viewer gameplay details and runtime diagnostics expanded state. Desktop dark operational UI. The "Gameplay Details" disclosure is open below the world board, showing formal gameplay summary, accepted intent, goal execution state, capability economics, recommended action, available gameplay actions, recent gameplay feedback. Runtime Diagnostics is also visible but visually secondary, using compact badges and expandable raw diagnostics. Preserve actual oasis7 viewer language: accepted intent, goal execution, blocker, next step, capability economics, runtime diagnostics, auth/session truth, recent events. This is a power-user inspection state, not the first player view. Keep information dense but organized, with clear section rhythm and no raw JSON wall dominating the viewport.
```

- Acceptance criteria:
  - Gameplay meaning remains above raw diagnostics.
  - Details are scannable in sections, not a card pile with equal weight.
  - Diagnostic/raw JSON surfaces are collapsed or visually subordinate unless the state explicitly opens them.
- Implementation order: sixth, after primary command and map surfaces.
- Fixture/test needs:
  - Need fixture with gameplay details open, diagnostics open, recent events, action list, and one feedback item.
  - Need query params or test API commands to open `#viewer-gameplay-details` and `#viewer-diagnostics-panel`.

### 7. Hosted Login / Access Gate

- Recommended output: `.pm/tasks/viewer-target-hosted-login-gate.png`
- Viewport: 1440 x 1000 desktop and later 390 x 844 if auth work proceeds
- Existing reference: none in current `.pm/tasks/viewer-*.png`
- Prompt:

```text
Create a high-fidelity target image for the oasis7 software-safe viewer hosted login/access gate state. The underlying viewer shell is visible but deemphasized; a focused access panel explains player session entry for hosted public join. Show real available fields only: player/session entry, challenge/approval or login fields if present in the viewer, error/retry notice area, continue/release session style actions. It must feel like a secure game access checkpoint, not a crypto wallet marketing page. Dark viewer palette, clear focus ring, compact labels, accessible modal spacing, no fake wallet balances, no token transfer promises, no fictional signup benefits.
```

- Acceptance criteria:
  - Login/access gate is clearly modal/focused and keyboard-readable.
  - Underlying gameplay shell is not mistaken for active command area.
  - Error/retry state has a reserved place and does not shift layout badly.
- Implementation order: seventh, unless hosted login is in the release-critical path.
- Fixture/test needs:
  - Need fixture/query state forcing `shouldShowHostedLoginGate()` true for hosted public join.
  - Need screenshot state for error/retry-after variant if implemented.

### 8. Empty-World Recovery

- Recommended output: `.pm/tasks/viewer-target-empty-world-recovery.png`
- Viewport: 1440 x 1000 desktop, optional mobile after desktop acceptance
- Existing reference: `.pm/tasks/viewer-focus-smoke-2026-06-18.png` is a rejected non-comparable smoke, useful only as what not to use for selected-agent acceptance
- Prompt:

```text
Create a high-fidelity target image for the oasis7 viewer empty-world recovery state. Desktop command desk with no agents or locations available yet, pending/empty lists, world board showing intentional empty or syncing state, and right command panel explaining that an agent must be selected before commands unlock. If the runtime publishes a recovery blocker, show it as the primary callout with a clear next step such as request snapshot or recover entities. The page should not look broken or like the selected-agent focus design; it should communicate "waiting / recover / select target" honestly. Dark industrial UI, subdued empty skeletons, one clear recovery action path, no fake map entities, no action receipt pretending a world delta exists.
```

- Acceptance criteria:
  - Empty state feels intentional and truthful, not a failed render.
  - It is visually distinct from selected-agent acceptance screenshots.
  - Recovery/next action is elevated above diagnostics.
- Implementation order: eighth, because it should not drive selected-agent visual acceptance but is important for resilience.
- Fixture/test needs:
  - Need explicit empty snapshot / pending snapshot fixture.
  - Need DOM marker for empty recovery state or blocker kind `runtime_snapshot_empty_entities`.

## Cross-State Rules

- Target images are visual direction and comparison evidence only; they do not replace browser smoke, functional tests, QA judgment, or runtime truth.
- Every generated image must be paired later with a native/browser screenshot from the same state and viewport.
- Generated targets must avoid fake features: no live token transfer form, no wallet marketing, no unreleased game mechanics, no invented agent abilities.
- Prefer English UI labels for current screenshot parity unless the implementation target is explicitly `locale=zh`; Chinese copy can be generated as a parallel set later.
- For implementation, stabilize fixtures before CSS polish so screenshots compare equivalent states.

## Implementation Order

1. Generate/approve `viewer-target-layout-desktop.png`.
2. Generate/approve `viewer-target-layout-mobile.png`.
3. Generate/approve `viewer-target-focus-selected-blocker-desktop.png`.
4. Generate/approve `viewer-target-focus-selected-blocker-mobile.png`.
5. Add fixture support for shell/command/details/login/empty states that are not yet reproducible.
6. Implement shell token/layout changes.
7. Implement focus desktop/mobile changes.
8. Implement command/chat and details/diagnostics changes.
9. Implement hosted-login and empty-recovery polish.
10. Capture matching browser screenshots and write gap notes for each target.

## Fixture Gaps

- Present: `pixel_world_visual_fixture=selected_blocker` covers selected-agent focus desktop/mobile.
- Needed: `viewer_visual_fixture=shell_selected_blocker` for full desktop/mobile shell with populated targets/stage/command.
- Needed: `viewer_visual_fixture=agent_chat_history` for selected-agent command/chat with message flow.
- Needed: `viewer_visual_fixture=gameplay_diagnostics_expanded` for expanded details/diagnostics and recent events.
- Needed: `viewer_visual_fixture=hosted_login_gate` for hosted public join/login gate.
- Needed: `viewer_visual_fixture=empty_world_recovery` for no entities / pending snapshot / recovery blocker.
