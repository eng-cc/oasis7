# Source Map

Use this file only when adapting `game-interaction-design`, explaining a checklist item, or deciding whether a new interaction rule belongs in the skill.

## Primary Inputs

- NN/g, "10 Usability Heuristics Applied to Video Games"
  - URL: https://www.nngroup.com/articles/usability-heuristics-applied-video-games/
  - Borrow for: visibility of status, player control, error prevention/recovery, consistency, recognition over recall, feedback loops.
  - Do not borrow as: a replacement for real player-facing smoke evidence.

- Xbox Accessibility Guidelines
  - URL: https://learn.microsoft.com/en-us/gaming/accessibility/guidelines
  - Borrow for: designer/developer/tester-friendly accessibility prompts and player capability framing.
  - Do not borrow as: legal compliance or platform certification.

- Game Accessibility Guidelines
  - URL: https://gameaccessibilityguidelines.com/full-list/
  - Borrow for: tiered accessibility ideas around text, contrast, input, timing, and control remapping.
  - Do not borrow as: a full checklist for every oasis7 task; select only current-flow-relevant items.

- Material Design 3, "Interaction states"
  - URL: https://m3.material.io/foundations/interaction/states
  - Borrow for: state vocabulary such as hover, focus, pressed, dragged, selected, disabled, and loading.
  - Do not borrow as: Material visual style or component library mandate.

- Apple Human Interface Guidelines, "Game controls"
  - URL: https://developer.apple.com/design/human-interface-guidelines/game-controls
  - Borrow for: platform/input awareness, legibility, control clarity, and screen-space pressure.
  - Do not borrow as: universal viewer behavior outside Apple-specific contexts.

- Celia Hodent, "Cognitive Psychology Applied to UX in Video Games"
  - URL: https://celiahodent.com/cognitive-psychology-applied-to-user-experience-in-video-games/
  - Borrow for: attention, perception, memory load, and gameflow review prompts.
  - Do not borrow as: gameplay balance, QA release judgment, or implementation feasibility.

## Oasis7 Translation Rules

- Convert each outside principle into a question about a specific player flow.
- Keep accessibility prompts scoped to the current screen, device assumptions, and implementation claim.
- Require real interaction evidence before saying a flow is validated.
- Route rule semantics to `gameplay_designer`, implementation feasibility to `viewer_engineer`, and release confidence to `qa_engineer`.
