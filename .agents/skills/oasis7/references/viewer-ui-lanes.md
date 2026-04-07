# Viewer / UI Lanes For Local Provider Real-Play

This page covers the UI-facing side of `oasis7`.
It is intentionally split out from the main skill so the default operator path stays focused on “how to run Local Provider real play”.

## Viewer Contract

Read the real-play path in two layers:

- Local Provider execution layer: `headless_agent` or `player_parity`
- human observation layer: `debug_viewer`, with `software_safe` as the weak-graphics fallback

Operator rule:

- Local Provider real-play does not require a Viewer to keep running
- `headless_agent` is the default regression/server lane
- `player_parity` is the lane to use when a human is judging player feel
- the Viewer is an observer/debug surface, not the authority execution path

## Recommended UI-Adjacent Commands

### Headless smoke / regression

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh smoke \
  --execution-mode headless_agent
```

### Real play without depending on a browser

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --execution-mode headless_agent \
  --reuse-bridge \
  --skip-agent-setup \
  --no-open-browser
```

### Player-feel / producer / QA run

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --execution-mode player_parity \
  --reuse-bridge \
  --skip-agent-setup
```

## Software-Safe Guidance

Use `software_safe` when:

- the browser lands on `SwiftShader`, `llvmpipe`, software renderer, or no usable WebGL path
- you need a DOM-first observer/debug lane
- you are validating minimal closure rather than visual quality
- a non-visual operator/model still needs to read state, select targets, or drive `play/pause/step`

Use `auto` by default for player-facing Viewer runs.
Force `software_safe` only when you need the fallback/observer lane.

Viewer URL hint:

- if the printed Viewer URL already has query params, append `&render_mode=software_safe`
- otherwise append `?render_mode=software_safe`

What `software_safe` is good for today:

- connection/status verification
- target selection
- DOM-readable state inspection
- `play/pause/step`
- evidence capture and QA smoke

What `software_safe` is not:

- not the primary player-experience mode
- not the visual-quality signoff path
- not the authority execution lane for Local Provider

## Current Local Provider UI Boundary

- in current `runtime live + provider_loopback_http` real-play, `agent_chat` and `prompt_control` should be treated as observer/debug surfaces, not an end-to-end supported player-authority path
- for QA-specific software-safe prompt/chat evidence, prefer `./scripts/viewer-software-safe-chat-regression.sh`
- for deeper Viewer/testing guidance, see `testing-manual.md` and the Viewer manual
